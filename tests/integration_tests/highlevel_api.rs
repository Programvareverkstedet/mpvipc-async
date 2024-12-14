use super::util::{get_test_asset, spawn_mpv};

use mpvipc_async::{
    MpvError, MpvExt, PlaylistAddOptions, PlaylistAddTypeOptions, SeekOptions, Switch,
};

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_seek() -> Result<(), MpvError> {
    let (mut proc, mpv) = spawn_mpv(false).await.unwrap();
    mpv.playlist_add(
        &get_test_asset("black-background-30s-480p.mp4"),
        PlaylistAddTypeOptions::File,
        PlaylistAddOptions::Append,
    )
    .await?;

    mpv.set_playback(Switch::On).await?;
    mpv.set_playback(Switch::Off).await?;

    // TODO: wait for property "seekable" to be true

    mpv.seek(10.0, SeekOptions::Relative).await?;
    let time_pos: f64 = mpv.get_property("time-pos").await?.unwrap();
    assert_eq!(time_pos, 10.0);

    mpv.seek(5.0, SeekOptions::Relative).await?;
    let time_pos: f64 = mpv.get_property("time-pos").await?.unwrap();
    assert_eq!(time_pos, 15.0);

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();

    Ok(())
}
