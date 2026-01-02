use std::{path::Path, time::Duration};

use thiserror::Error;
use tokio::{
    process::{Child, Command},
    time::{sleep, timeout},
};
use tokio_stream::StreamExt;

use mpvipc_async::{Event, Mpv, MpvError, MpvExt, Property, parse_property};

#[cfg(target_family = "unix")]
pub async fn spawn_headless_mpv() -> Result<(Child, Mpv), MpvError> {
    let socket_path_str = format!("/tmp/mpv-ipc-{}", uuid::Uuid::new_v4());
    let socket_path = Path::new(&socket_path_str);

    // TODO: Verify that `mpv` exists in `PATH``
    let process_handle = Command::new("mpv")
        .arg("--no-config")
        .arg("--idle")
        .arg("--no-video")
        .arg("--no-audio")
        .arg(format!(
            "--input-ipc-server={}",
            &socket_path.to_str().unwrap()
        ))
        .kill_on_drop(true)
        .spawn()
        .expect("Failed to start mpv");

    timeout(Duration::from_millis(1000), async {
        while !&socket_path.exists() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| {
        MpvError::MpvSocketConnectionError(format!(
            "Failed to create mpv socket at {:?}, timed out waiting for socket file to be created",
            &socket_path
        ))
    })?;

    let mpv = Mpv::connect(socket_path.to_str().unwrap()).await?;
    Ok((process_handle, mpv))
}

pub const MPV_CHANNEL_ID: u64 = 1337;

#[derive(Error, Debug)]
pub enum PropertyCheckingThreadError {
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
pub fn create_interruptable_event_property_checking_thread<T>(
    mpv: Mpv,
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
        let mut events = mpv.get_event_stream().await;

        loop {
            tokio::select! {
                event = events.next() => {
                    match event {
                        Some(Ok(event)) => {
                            match event {
                                Event::PropertyChange { id: Some(MPV_CHANNEL_ID), name, data } => {
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

pub async fn check_property_thread_result(
    handle: tokio::task::JoinHandle<Result<(), PropertyCheckingThreadError>>,
) -> Result<(), MpvError> {
    timeout(Duration::from_millis(500), handle)
        .await
        .map_err(|_| {
            MpvError::InternalConnectionError("Event checking thread timed out".to_owned())
        })?
        .map_err(|_| {
            MpvError::InternalConnectionError("Event checking thread panicked".to_owned())
        })?
        .map_err(|err| match err {
            PropertyCheckingThreadError::UnexpectedPropertyError(property) => {
                MpvError::Other(format!("Unexpected property: {:?}", property))
            }
            PropertyCheckingThreadError::MpvError(err) => err,
        })
}

/// This helper function will gracefully shut down both the event checking thread and the mpv process.
/// It will also return an error if the event checking thread happened to panic, or if it times out
/// The timeout is hardcoded to 500ms.
pub async fn graceful_shutdown(
    cancellation_token: tokio_util::sync::CancellationToken,
    handle: tokio::task::JoinHandle<Result<(), PropertyCheckingThreadError>>,
    mpv: Mpv,
    mut proc: tokio::process::Child,
) -> Result<(), MpvError> {
    cancellation_token.cancel();

    check_property_thread_result(handle).await?;

    mpv.kill().await?;
    proc.wait().await.map_err(|err| {
        MpvError::InternalConnectionError(format!(
            "Failed to wait for mpv process to exit: {}",
            err
        ))
    })?;

    Ok(())
}
