use mpvipc::MpvExt;

use super::*;

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_get_mpv_version() {
    let (mut proc, mpv) = spawn_headless_mpv().await.unwrap();
    let version: String = mpv.get_property("mpv-version").await.unwrap();
    assert!(version.starts_with("mpv"));

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();
}

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_set_property() {
    let (mut proc, mpv) = spawn_headless_mpv().await.unwrap();
    mpv.set_property("pause", true).await.unwrap();
    let paused: bool = mpv.get_property("pause").await.unwrap();
    assert!(paused);

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();
}
