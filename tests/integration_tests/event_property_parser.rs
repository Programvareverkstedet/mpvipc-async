use futures::{stream::StreamExt, Stream};
use mpvipc_async::{parse_property, Event, Mpv, MpvError, MpvExt, Property};
use thiserror::Error;
use tokio::time::sleep;
use tokio::time::{timeout, Duration};

use test_log::test;

use super::*;

const MPV_CHANNEL_ID: u64 = 1337;

#[derive(Error, Debug)]
enum PropertyCheckingThreadError {
    #[error("Unexpected property: {0:?}")]
    UnexpectedPropertyError(Property),

    #[error(transparent)]
    MpvError(#[from] MpvError),
}

/// This function will create an ongoing tokio task that collects [`Event::PropertyChange`] events,
/// and parses them into [`Property`]s. It will then run the property through the provided
/// closure, and return an error if the closure returns false.
///
/// The returned cancellation token can be used to stop the task.
fn create_interruptable_event_property_checking_thread<T>(
    mut events: impl Stream<Item = Result<Event, MpvError>> + Unpin + Send + 'static,
    on_property: T,
) -> (
    tokio::task::JoinHandle<Result<(), PropertyCheckingThreadError>>,
    tokio_util::sync::CancellationToken,
)
where
    T: Fn(Property) -> bool + Send + 'static,
{
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let cancellation_token_clone = cancellation_token.clone();
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                event = events.next() => {
                    match event {
                        Some(Ok(event)) => {
                            match event {
                                Event::PropertyChange { id: MPV_CHANNEL_ID, name, data } => {
                                    let property = parse_property(&name, data).unwrap();
                                    if !on_property(property.clone()) {
                                        return Err(PropertyCheckingThreadError::UnexpectedPropertyError(property))
                                    }
                                }
                                _ => {
                                    log::trace!("Received unrelated event, ignoring: {:?}", event);
                                }
                            }
                        }
                        Some(Err(err)) => return Err(err.into()),
                        None => return Ok(()),
                    }
                }
                _ = cancellation_token_clone.cancelled() => return Ok(()),
            }
        }
    });

    (handle, cancellation_token)
}

/// This helper function will gracefully shut down both the event checking thread and the mpv process.
/// It will also return an error if the event checking thread happened to panic, or if it times out
/// The timeout is hardcoded to 500ms.
async fn graceful_shutdown(
    cancellation_token: tokio_util::sync::CancellationToken,
    handle: tokio::task::JoinHandle<Result<(), PropertyCheckingThreadError>>,
    mpv: Mpv,
    mut proc: tokio::process::Child,
) -> Result<(), MpvError> {
    cancellation_token.cancel();

    match timeout(Duration::from_millis(500), handle).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(err))) => match err {
            PropertyCheckingThreadError::UnexpectedPropertyError(property) => {
                return Err(MpvError::Other(format!(
                    "Unexpected property: {:?}",
                    property
                )));
            }
            PropertyCheckingThreadError::MpvError(err) => return Err(err),
        },
        Ok(Err(_)) => {
            return Err(MpvError::InternalConnectionError(
                "Event checking thread timed out".to_owned(),
            ));
        }
        Err(_) => {
            return Err(MpvError::InternalConnectionError(
                "Event checking thread panicked".to_owned(),
            ));
        }
    }

    mpv.kill().await?;
    proc.wait().await.map_err(|err| {
        MpvError::InternalConnectionError(format!(
            "Failed to wait for mpv process to exit: {}",
            err
        ))
    })?;

    Ok(())
}

/// Test correct parsing of different values of the "pause" property
#[test(tokio::test)]
#[cfg(target_family = "unix")]
async fn test_highlevel_event_pause() -> Result<(), MpvError> {
    let (proc, mpv) = spawn_mpv(true).await?;

    mpv.observe_property(MPV_CHANNEL_ID, "pause").await?;

    let events = mpv.get_event_stream().await;
    let (handle, cancellation_token) =
        create_interruptable_event_property_checking_thread(events, |property| match property {
            Property::Pause(_) => {
                log::debug!("{:?}", property);
                true
            }
            _ => false,
        });

    sleep(Duration::from_millis(5)).await;
    mpv.set_property("pause", false).await?;
    sleep(Duration::from_millis(5)).await;
    mpv.set_property("pause", true).await?;
    sleep(Duration::from_millis(5)).await;

    graceful_shutdown(cancellation_token, handle, mpv, proc).await?;

    Ok(())
}

/// Test correct parsing of different values of the "volume" property
#[test(tokio::test)]
#[cfg(target_family = "unix")]
async fn test_highlevel_event_volume() -> Result<(), MpvError> {
    let (proc, mpv) = spawn_mpv(true).await?;

    mpv.observe_property(1337, "volume").await?;
    let events = mpv.get_event_stream().await;
    let (handle, cancellation_token) =
        create_interruptable_event_property_checking_thread(events, |property| match property {
            Property::Volume(_) => {
                log::trace!("{:?}", property);
                true
            }
            _ => false,
        });

    sleep(Duration::from_millis(5)).await;
    mpv.set_property("volume", 100.0).await?;
    sleep(Duration::from_millis(5)).await;
    mpv.set_property("volume", 40).await?;
    sleep(Duration::from_millis(5)).await;
    mpv.set_property("volume", 0.0).await?;
    sleep(Duration::from_millis(5)).await;

    graceful_shutdown(cancellation_token, handle, mpv, proc).await?;

    Ok(())
}

/// Test correct parsing of different values of the "mute" property
#[test(tokio::test)]
#[cfg(target_family = "unix")]
async fn test_highlevel_event_mute() -> Result<(), MpvError> {
    let (proc, mpv) = spawn_mpv(true).await?;

    mpv.observe_property(1337, "mute").await?;
    let events = mpv.get_event_stream().await;
    let (handle, cancellation_token) =
        create_interruptable_event_property_checking_thread(events, |property| match property {
            Property::Mute(_) => {
                log::trace!("{:?}", property);
                true
            }
            _ => false,
        });

    sleep(Duration::from_millis(5)).await;
    mpv.set_property("mute", true).await?;
    sleep(Duration::from_millis(5)).await;
    mpv.set_property("mute", false).await?;
    sleep(Duration::from_millis(5)).await;

    graceful_shutdown(cancellation_token, handle, mpv, proc).await?;

    Ok(())
}

/// Test correct parsing of different values of the "duration" property
#[test(tokio::test)]
#[cfg(target_family = "unix")]
async fn test_highlevel_event_duration() -> Result<(), MpvError> {
    let (proc, mpv) = spawn_mpv(true).await?;

    mpv.observe_property(1337, "duration").await?;

    let events = mpv.get_event_stream().await;
    let (handle, cancellation_token) =
        create_interruptable_event_property_checking_thread(events, |property| match property {
            Property::Duration(_) => {
                log::trace!("{:?}", property);
                true
            }
            _ => false,
        });

    sleep(Duration::from_millis(5)).await;
    mpv.set_property("pause", true).await?;
    sleep(Duration::from_millis(5)).await;
    mpv.set_property("pause", false).await?;
    sleep(Duration::from_millis(5)).await;

    graceful_shutdown(cancellation_token, handle, mpv, proc).await?;

    Ok(())
}
