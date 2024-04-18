use std::{panic, time::Duration};

use futures::{stream::FuturesUnordered, SinkExt, StreamExt};
use mpvipc::{Error, ErrorCode, Mpv, Playlist, PlaylistEntry};
use serde_json::{json, Value};
use test_log::test;
use tokio::{net::UnixStream, task::JoinHandle};
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

fn test_socket(answers: Vec<String>) -> (UnixStream, JoinHandle<Result<(), LinesCodecError>>) {
    let (socket, server) = UnixStream::pair().unwrap();
    let join_handle = tokio::spawn(async move {
        let mut framed = Framed::new(socket, LinesCodec::new());
        for answer in answers {
            framed.next().await;
            framed.send(answer).await?;
        }
        Ok(())
    });

    (server, join_handle)
}

#[test(tokio::test)]
async fn test_get_property_successful() {
    let (server, join_handle) = test_socket(vec![
        json!({ "data": 100.0, "request_id": 0, "error": "success" }).to_string(),
    ]);

    let mpv = Mpv::connect_socket(server).await.unwrap();
    let volume: f64 = mpv.get_property("volume").await.unwrap();

    assert_eq!(volume, 100.0);
    join_handle.await.unwrap().unwrap();
}

#[test(tokio::test)]
async fn test_get_property_broken_pipe() {
    let (server, join_handle) = test_socket(vec![]);

    let mpv = Mpv::connect_socket(server).await.unwrap();
    let maybe_volume = mpv.get_property::<f64>("volume").await;

    assert_eq!(
        maybe_volume,
        Err(Error(ErrorCode::ConnectError(
            "Broken pipe (os error 32)".to_owned()
        )))
    );
    join_handle.await.unwrap().unwrap();
}

#[test(tokio::test)]
async fn test_get_property_wrong_type() {
    let (server, join_handle) = test_socket(vec![
        json!({ "data": 100.0, "request_id": 0, "error": "success" }).to_string(),
    ]);

    let mpv = Mpv::connect_socket(server).await.unwrap();
    let maybe_volume = mpv.get_property::<bool>("volume").await;

    assert_eq!(maybe_volume, Err(Error(ErrorCode::ValueDoesNotContainBool)));
    join_handle.await.unwrap().unwrap();
}

#[test(tokio::test)]
async fn test_get_property_error() {
    let (server, join_handle) = test_socket(vec![
        json!({ "error": "property unavailable", "request_id": 0 }).to_string(),
    ]);

    let mpv = Mpv::connect_socket(server).await.unwrap();
    let maybe_volume = mpv.get_property::<f64>("volume").await;

    assert_eq!(
        maybe_volume,
        Err(Error(ErrorCode::MpvError(
            "property unavailable".to_owned()
        )))
    );

    join_handle.await.unwrap().unwrap();
}

#[test(tokio::test)]
async fn test_get_property_simultaneous_requests() {
    let (socket, server) = UnixStream::pair().unwrap();
    let mpv_handle: JoinHandle<Result<(), LinesCodecError>> = tokio::spawn(async move {
        let mut framed = Framed::new(socket, LinesCodec::new());

        while let Some(request) = framed.next().await {
            match serde_json::from_str::<Value>(&request.unwrap()) {
                Ok(json) => {
                    let property = json["command"][1].as_str().unwrap();
                    log::info!("Received request for property: {:?}", property);
                    match property {
                        "volume" => {
                            let response =
                                json!({ "data": 100.0, "request_id": 0, "error": "success" })
                                    .to_string();
                            framed.send(response).await.unwrap();
                        }
                        "pause" => {
                            let response =
                                json!({ "data": true, "request_id": 0, "error": "success" })
                                    .to_string();
                            framed.send(response).await.unwrap();
                        }
                        _ => {
                            let response =
                                json!({ "error": "property unavailable", "request_id": 0 })
                                    .to_string();
                            framed.send(response).await.unwrap();
                        }
                    }
                }
                Err(_) => {}
            }
        }

        Ok(())
    });

    let mpv = Mpv::connect_socket(server).await.unwrap();

    let mpv_clone_1 = mpv.clone();
    let mpv_poller_1 = tokio::spawn(async move {
        loop {
            let volume: f64 = mpv_clone_1.get_property("volume").await.unwrap();
            assert_eq!(volume, 100.0);
        }
    });

    let mpv_clone_2 = mpv.clone();
    let mpv_poller_2 = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1)).await;
            let paused: bool = mpv_clone_2.get_property("pause").await.unwrap();
            assert_eq!(paused, true);
        }
    });

    let mpv_clone_3 = mpv.clone();
    let mpv_poller_3 = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(2)).await;
            let maybe_volume = mpv_clone_3.get_property::<f64>("nonexistent").await;
            assert_eq!(
                maybe_volume,
                Err(Error(ErrorCode::MpvError(
                    "property unavailable".to_owned()
                )))
            );
        }
    });

    let mut tasks = FuturesUnordered::new();
    tasks.push(mpv_handle);
    tasks.push(mpv_poller_1);
    tasks.push(mpv_poller_2);
    tasks.push(mpv_poller_3);

    if tokio::time::timeout(Duration::from_millis(200), tasks.next())
        .await
        .is_ok()
    {
        panic!("One of the pollers quit unexpectedly");
    };
}

#[test(tokio::test)]
async fn test_get_playlist() {
    let expected = Playlist(vec![
        PlaylistEntry {
            id: 0,
            filename: "file1".to_string(),
            title: "title1".to_string(),
            current: false,
        },
        PlaylistEntry {
            id: 1,
            filename: "file2".to_string(),
            title: "title2".to_string(),
            current: true,
        },
        PlaylistEntry {
            id: 2,
            filename: "file3".to_string(),
            title: "title3".to_string(),
            current: false,
        },
    ]);

    let (server, join_handle) = test_socket(vec![json!({
      "data": expected.0.iter().map(|entry| {
        json!({
          "filename": entry.filename,
          "title": entry.title,
          "current": entry.current
        })
      }).collect::<Vec<Value>>(),
      "request_id": 0,
      "error": "success"
    })
    .to_string()]);

    let mpv = Mpv::connect_socket(server).await.unwrap();
    let playlist = mpv.get_playlist().await.unwrap();

    assert_eq!(playlist, expected);
    join_handle.await.unwrap().unwrap();
}

#[test(tokio::test)]
async fn test_get_playlist_empty() {
    let (server, join_handle) = test_socket(vec![
        json!({ "data": [], "request_id": 0, "error": "success" }).to_string(),
    ]);

    let mpv = Mpv::connect_socket(server).await.unwrap();
    let playlist = mpv.get_playlist().await.unwrap();

    assert_eq!(playlist, Playlist(vec![]));
    join_handle.await.unwrap().unwrap();
}
