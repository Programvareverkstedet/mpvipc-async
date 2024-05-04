use mpvipc::{Mpv, MpvError, MpvExt};

#[tokio::main]
async fn main() -> Result<(), MpvError> {
    env_logger::init();

    let mpv = Mpv::connect("/tmp/mpv.sock").await?;

    let meta = mpv.get_metadata().await?;
    println!("metadata: {:?}", meta);

    let playlist = mpv.get_playlist().await?;
    println!("playlist: {:?}", playlist);

    let playback_time: Option<f64> = mpv.get_property("playback-time").await?;
    println!("playback-time: {:?}", playback_time);

    Ok(())
}
