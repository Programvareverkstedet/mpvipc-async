use std::{path::Path, time::Duration};

use mpvipc::{Mpv, MpvError};
use tokio::{
    process::{Child, Command},
    time::{sleep, timeout},
};

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

    timeout(Duration::from_millis(500), async {
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
