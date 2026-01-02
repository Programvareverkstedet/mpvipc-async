use std::time::Duration;

use test_log::test;
use tokio::time::sleep;

use mpvipc_async::{MpvError, MpvExt, Property};

use super::*;

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_get_mpv_version() -> Result<(), MpvError> {
    let (mut proc, mpv) = spawn_headless_mpv().await.unwrap();
    let version: String = mpv.get_property("mpv-version").await?.unwrap();
    assert!(version.starts_with("mpv"));

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();

    Ok(())
}

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_set_property() -> Result<(), MpvError> {
    let (mut proc, mpv) = spawn_headless_mpv().await.unwrap();
    mpv.set_property("pause", true).await.unwrap();
    let paused: bool = mpv.get_property("pause").await?.unwrap();
    assert!(paused);

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();

    Ok(())
}

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_get_unavailable_property() -> Result<(), MpvError> {
    let (mut proc, mpv) = spawn_headless_mpv().await.unwrap();
    let time_pos = mpv.get_property::<f64>("time-pos").await;
    assert_eq!(time_pos, Ok(None));

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();

    Ok(())
}

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_get_nonexistent_property() -> Result<(), MpvError> {
    let (mut proc, mpv) = spawn_headless_mpv().await.unwrap();
    let nonexistent = mpv.get_property::<f64>("nonexistent").await;

    match nonexistent {
        Err(MpvError::MpvError { message, .. }) => {
            assert_eq!(message, "property not found");
        }
        _ => panic!("Unexpected result: {:?}", nonexistent),
    }

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();

    Ok(())
}

#[test(tokio::test)]
#[cfg(target_family = "unix")]
async fn test_unobserve_property() -> Result<(), MpvError> {
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
    mpv.set_property("pause", true).await?;
    sleep(Duration::from_millis(5)).await;

    cancellation_token.cancel();
    check_property_thread_result(handle).await?;

    mpv.unobserve_property(MPV_CHANNEL_ID).await?;

    let (handle, cancellation_token) =
        create_interruptable_event_property_checking_thread(mpv.clone(), |_property| {
            // We should not receive any properties after unobserving
            false
        });

    sleep(Duration::from_millis(5)).await;
    mpv.set_property("pause", false).await?;
    sleep(Duration::from_millis(5)).await;

    graceful_shutdown(cancellation_token, handle, mpv, proc).await?;

    Ok(())
}
