use std::{path::Path, time::Duration};

use mpvipc_async::{Mpv, MpvError};
use tokio::{
    process::{Child, Command},
    time::{sleep, timeout},
};

pub fn assert_test_assets_exist() {
    let test_data_dir = Path::new("test_assets");
    if !test_data_dir.exists()
        || !test_data_dir.is_dir()
        // `.gitkeep` should always be present, so there should be at least 2 entries
        || test_data_dir.read_dir().unwrap().count() <= 1
    {
        panic!(
            "Test assets directory not found at {:?}, please run `./setup_test_assets.sh`",
            test_data_dir
        );
    }
}

#[inline]
pub fn get_test_assets_dir() -> &'static Path {
    Path::new("test_assets")
}

pub fn get_test_asset(file_name: &str) -> String {
    assert_test_assets_exist();

    let test_assets_dir = get_test_assets_dir();
    let file_path = test_assets_dir.join(file_name);
    file_path.to_str().unwrap().to_string()
}

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
