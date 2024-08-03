use std::{panic, time::Duration};

use futures::{stream::FuturesUnordered, SinkExt, StreamExt};
use mpvipc_async::{Mpv, MpvError};
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
async fn test_set_property_successful() -> Result<(), MpvError> {
    let (server, join_handle) = test_socket(vec![
        json!({ "data": null, "request_id": 0, "error": "success" }).to_string(),
    ]);

    let mpv = Mpv::connect_socket(server).await?;
    mpv.set_property("volume", 64.0).await?;

    join_handle.await.unwrap().unwrap();

    Ok(())
}

#[test(tokio::test)]
async fn test_set_property_broken_pipe() -> Result<(), MpvError> {
    let (server, join_handle) = test_socket(vec![]);

    let mpv = Mpv::connect_socket(server).await?;
    let maybe_set_volume = mpv.set_property("volume", 64.0).await;

    assert_eq!(
        maybe_set_volume,
        Err(MpvError::MpvSocketConnectionError(
            "Broken pipe (os error 32)".to_string()
        ))
    );

    join_handle.await.unwrap().unwrap();

    Ok(())
}

#[test(tokio::test)]
async fn test_set_property_wrong_type() -> Result<(), MpvError> {
    let (server, join_handle) = test_socket(vec![
        json!({"request_id":0,"error":"unsupported format for accessing property"}).to_string(),
    ]);

    let mpv = Mpv::connect_socket(server).await?;
    let maybe_volume = mpv.set_property::<bool>("volume", true).await;

    assert_eq!(
        maybe_volume,
        Err(MpvError::MpvError(
            "unsupported format for accessing property".to_string()
        ))
    );

    join_handle.await.unwrap().unwrap();

    Ok(())
}

#[test(tokio::test)]
async fn test_get_property_error() -> Result<(), MpvError> {
    let (server, join_handle) = test_socket(vec![
        json!({"request_id":0,"error":"property not found"}).to_string(),
    ]);

    let mpv = Mpv::connect_socket(server).await?;
    let maybe_volume = mpv.set_property("nonexistent", true).await;

    assert_eq!(
        maybe_volume,
        Err(MpvError::MpvError("property not found".to_string()))
    );

    join_handle.await.unwrap().unwrap();

    Ok(())
}

#[test(tokio::test)]
async fn test_set_property_simultaneous_requests() {
    let (socket, server) = UnixStream::pair().unwrap();
    let mpv_handle: JoinHandle<Result<(), LinesCodecError>> = tokio::spawn(async move {
        let mut framed = Framed::new(socket, LinesCodec::new());

        while let Some(request) = framed.next().await {
            if let Ok(json) = serde_json::from_str::<Value>(&request.unwrap()) {
                let property = json["command"][1].as_str().unwrap();
                let value = &json["command"][2];
                log::info!(
                    "Received set property command: {:?} => {:?}",
                    property,
                    value
                );
                match property {
                    "volume" => {
                        let response = json!({ "request_id": 0, "error": "success" }).to_string();
                        framed.send(response).await.unwrap();
                    }
                    "pause" => {
                        let response = json!({ "request_id": 0, "error": "success" }).to_string();
                        framed.send(response).await.unwrap();
                    }
                    _ => {
                        let response =
                            json!({ "error":"property not found", "request_id": 0 }).to_string();
                        framed.send(response).await.unwrap();
                    }
                }
            }
        }

        Ok(())
    });

    let mpv = Mpv::connect_socket(server).await.unwrap();

    let mpv_clone_1 = mpv.clone();
    let mpv_poller_1 = tokio::spawn(async move {
        loop {
            let status = mpv_clone_1.set_property("volume", 100).await;
            match status {
                Ok(()) => {}
                _ => panic!("Unexpected result: {:?}", status),
            }
        }
    });

    let mpv_clone_2 = mpv.clone();
    let mpv_poller_2 = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1)).await;
            let status = mpv_clone_2.set_property("pause", false).await;
            match status {
                Ok(()) => {}
                _ => panic!("Unexpected result: {:?}", status),
            }
        }
    });

    let mpv_clone_3 = mpv.clone();
    let mpv_poller_3 = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(2)).await;
            let maybe_volume = mpv_clone_3.set_property("nonexistent", "a").await;
            match maybe_volume {
                Err(MpvError::MpvError(err)) => {
                    assert_eq!(err, "property not found");
                }
                _ => panic!("Unexpected result: {:?}", maybe_volume),
            }
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
