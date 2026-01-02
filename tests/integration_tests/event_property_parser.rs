use test_log::test;
use tokio::time::Duration;
use tokio::time::sleep;

use mpvipc_async::{MpvError, MpvExt, Property};

use super::*;

/// Test correct parsing of different values of the "pause" property
#[test(tokio::test)]
#[cfg(target_family = "unix")]
async fn test_highlevel_event_pause() -> Result<(), MpvError> {
    let (proc, mpv) = spawn_headless_mpv().await?;

    mpv.observe_property(MPV_CHANNEL_ID, "pause").await?;

    let (handle, cancellation_token) = create_interruptable_event_property_checking_thread(
        mpv.clone(),
        |property| match property {
            Property::Pause(_) => {
                log::debug!("{:?}", property);
                true
            }
            _ => false,
        },
    );

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
    let (proc, mpv) = spawn_headless_mpv().await?;

    mpv.observe_property(MPV_CHANNEL_ID, "volume").await?;
    let (handle, cancellation_token) = create_interruptable_event_property_checking_thread(
        mpv.clone(),
        |property| match property {
            Property::Volume(_) => {
                log::trace!("{:?}", property);
                true
            }
            _ => false,
        },
    );

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
    let (proc, mpv) = spawn_headless_mpv().await?;

    mpv.observe_property(MPV_CHANNEL_ID, "mute").await?;
    let (handle, cancellation_token) = create_interruptable_event_property_checking_thread(
        mpv.clone(),
        |property| match property {
            Property::Mute(_) => {
                log::trace!("{:?}", property);
                true
            }
            _ => false,
        },
    );

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
    let (proc, mpv) = spawn_headless_mpv().await?;

    mpv.observe_property(MPV_CHANNEL_ID, "duration").await?;

    let (handle, cancellation_token) = create_interruptable_event_property_checking_thread(
        mpv.clone(),
        |property| match property {
            Property::Duration(_) => {
                log::trace!("{:?}", property);
                true
            }
            _ => false,
        },
    );

    sleep(Duration::from_millis(5)).await;
    mpv.set_property("pause", true).await?;
    sleep(Duration::from_millis(5)).await;
    mpv.set_property("pause", false).await?;
    sleep(Duration::from_millis(5)).await;

    graceful_shutdown(cancellation_token, handle, mpv, proc).await?;

    Ok(())
}
