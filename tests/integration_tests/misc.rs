use mpvipc_async::{MpvError, MpvExt};

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
    assert_eq!(nonexistent, Err(MpvError::MpvError("property not found".to_string())));

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();

    Ok(())
}