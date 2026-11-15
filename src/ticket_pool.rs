//! Fixed-capacity pool of numbered tickets.
//!
//! Used for tracking in-flight requests in the inner ipc loop.

use std::sync::{
    Arc,
    atomic::{
        AtomicU64, AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
    },
};

use tokio::sync::Semaphore;

const WORD_BITS: usize = u64::BITS as usize;

/// A pool of `WORDS * 64` tickets.
pub(crate) struct TicketPool<const WORDS: usize> {
    /// Bitmaps of used slots
    in_use: [AtomicU64; WORDS],
    /// Semaphore counting free slots.
    free: Semaphore,
    /// Next slot to check.
    cursor: AtomicUsize,
}

impl<const WORDS: usize> TicketPool<WORDS> {
    pub(crate) const CAPACITY: usize = WORDS * WORD_BITS;

    pub(crate) fn new() -> Self {
        const { assert!(WORDS > 0) };
        TicketPool {
            in_use: [const { AtomicU64::new(0) }; WORDS],
            free: Semaphore::new(Self::CAPACITY),
            cursor: AtomicUsize::new(0),
        }
    }

    /// The slot of a rendered ticket number, if it is a valid one.
    pub(crate) fn slot_of(rendered: u64) -> Option<usize> {
        (1..=Self::CAPACITY as u64)
            .contains(&rendered)
            .then(|| rendered as usize - 1)
    }

    fn bit(slot: usize) -> (usize, u64) {
        (slot / WORD_BITS, 1 << (slot % WORD_BITS))
    }

    pub(crate) fn slot_in_use(&self, slot: usize) -> bool {
        let (word, bit) = Self::bit(slot);
        self.in_use[word].load(Acquire) & bit != 0
    }

    /// The first free slot at or after `start`, wrapping around.
    fn next_free_slot(&self, start: usize) -> Option<usize> {
        (0..=WORDS).find_map(|step| {
            let word = (start / WORD_BITS + step) % WORDS;
            let below_start = if step == 0 { start % WORD_BITS } else { 0 };
            let free = !self.in_use[word].load(Acquire) & (u64::MAX << below_start);
            (free != 0).then(|| word * WORD_BITS + free.trailing_zeros() as usize)
        })
    }

    /// Issue a new ticket when one is available, waiting if necessary.
    pub(crate) async fn issue(self: &Arc<Self>) -> Ticket<WORDS> {
        let permit = self.free.acquire().await;
        permit
            .expect("the ticket semaphore closed unexpectedly")
            .forget();

        loop {
            let start = self.cursor.load(Relaxed);
            let slot = self
                .next_free_slot(start)
                .expect("semaphore reported free slot that didn't exist");

            let (word, bit) = Self::bit(slot);
            if self.in_use[word].fetch_or(bit, Acquire) & bit == 0 {
                self.cursor.store((slot + 1) % Self::CAPACITY, Relaxed);
                return Ticket {
                    pool: self.clone(),
                    slot,
                };
            }
        }
    }
}

/// Occupies a slot in a [`TicketPool`] until dropped.
pub(crate) struct Ticket<const WORDS: usize> {
    pool: Arc<TicketPool<WORDS>>,
    slot: usize,
}

impl<const WORDS: usize> Ticket<WORDS> {
    pub(crate) fn slot(&self) -> usize {
        self.slot
    }

    /// The ticket number. These start at 1 and go to `TicketPool::CAPACITY`.
    pub(crate) fn render(&self) -> u64 {
        self.slot as u64 + 1
    }
}

impl<const WORDS: usize> Drop for Ticket<WORDS> {
    fn drop(&mut self) {
        let (word, bit) = TicketPool::<WORDS>::bit(self.slot);
        self.pool.in_use[word].fetch_and(!bit, Release);
        self.pool.free.add_permits(1);
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;

    use super::*;

    fn try_issue<const WORDS: usize>(pool: &Arc<TicketPool<WORDS>>) -> Option<Ticket<WORDS>> {
        tokio::task::unconstrained(pool.issue()).now_or_never()
    }

    fn filled_pool<const WORDS: usize>(
        except: &[usize],
    ) -> (Arc<TicketPool<WORDS>>, Vec<Ticket<WORDS>>) {
        let pool = Arc::new(TicketPool::new());
        let mut tickets: Vec<_> = (0..TicketPool::<WORDS>::CAPACITY)
            .map(|_| try_issue(&pool).unwrap())
            .collect();
        tickets.retain(|ticket| !except.contains(&ticket.slot()));
        (pool, tickets)
    }

    #[test]
    fn starts_at_1() {
        for (rendered, slot) in [(0, None), (1, Some(0)), (64, Some(63)), (65, None)] {
            assert_eq!(TicketPool::<1>::slot_of(rendered), slot);
        }
    }

    #[tokio::test]
    async fn round_robin() {
        let pool = Arc::new(TicketPool::<1>::new());
        let first = pool.issue().await;
        assert_eq!((first.slot(), first.render()), (0, 1));
        assert_eq!(pool.issue().await.slot(), 1);
        drop(first);
        assert_eq!(pool.issue().await.slot(), 2);
        pool.cursor.store(63, Relaxed);
        assert_eq!(pool.issue().await.render(), 64);
        assert_eq!(pool.issue().await.slot(), 0);
    }

    #[tokio::test]
    async fn jumps_to_the_next_free_slot() {
        let (pool, _tickets) = filled_pool::<1>(&[3]);
        assert_eq!(try_issue(&pool).unwrap().slot(), 3);
        assert_eq!(pool.cursor.load(Relaxed), 4);
    }

    #[tokio::test]
    async fn searches_over_multiple_words() {
        let (earlier, later) = (7, 2 * WORD_BITS + 5);
        let (pool, _tickets) = filled_pool::<4>(&[earlier, later]);
        pool.cursor.store(1, Relaxed);
        let first = try_issue(&pool).unwrap();
        assert_eq!(first.slot(), earlier);
        let _second = try_issue(&pool)
            .filter(|ticket| ticket.slot() == later)
            .unwrap();
        assert!(try_issue(&pool).is_none());
        drop(first);
        assert_eq!(try_issue(&pool).unwrap().slot(), earlier);
    }

    #[tokio::test]
    async fn waits_for_free_slots() {
        let (pool, mut tickets) = filled_pool::<1>(&[]);
        assert!(try_issue(&pool).is_none());

        let freed_slot = tickets.remove(41).slot();
        assert_eq!(try_issue(&pool).unwrap().slot(), freed_slot);
    }
}
