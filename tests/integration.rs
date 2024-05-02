use mpvipc::{Error, Mpv, MpvExt};
use std::path::Path;
use tokio::{
    process::{Child, Command},
    time::{sleep, timeout, Duration},
};

#[cfg(target_family = "unix")]
async fn spawn_headless_mpv() -> Result<(Child, Mpv), Error> {
    let socket_path_str = format!("/tmp/mpv-ipc-{}", uuid::Uuid::new_v4());
    let socket_path = Path::new(&socket_path_str);

    let process_handle = Command::new("mpv")
        .arg("--no-config")
        .arg("--idle")
        .arg("--no-video")
        .arg("--no-audio")
        .arg(format!(
            "--input-ipc-server={}",
            &socket_path.to_str().unwrap()
        ))
        .spawn()
        .expect("Failed to start mpv");

    if timeout(Duration::from_millis(500), async {
        while !&socket_path.exists() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_err()
    {
        panic!("Failed to create mpv socket at {:?}", &socket_path);
    }

    let mpv = Mpv::connect(socket_path.to_str().unwrap()).await.unwrap();
    Ok((process_handle, mpv))
}

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

#[tokio::test]
#[cfg(target_family = "unix")]
async fn test_events() {
    use futures::stream::StreamExt;

    let (mut proc, mpv) = spawn_headless_mpv().await.unwrap();

    mpv.observe_property(1337, "pause").await.unwrap();

    let mut events = mpv.get_event_stream().await;
    let event_checking_thread = tokio::spawn(async move {
        loop {
            let event = events.next().await.unwrap().unwrap();
            if let (1337, property) = mpvipc::parse_event_property(event).unwrap() {
                assert_eq!(property, mpvipc::Property::Pause(true));
                break;
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    mpv.set_property("pause", true).await.unwrap();

    if tokio::time::timeout(
        tokio::time::Duration::from_millis(500),
        event_checking_thread,
    )
    .await
    .is_err()
    {
        panic!("Event checking thread timed out");
    }

    mpv.kill().await.unwrap();
    proc.kill().await.unwrap();
}
