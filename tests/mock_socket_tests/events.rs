use futures::{stream::StreamExt, SinkExt};
use mpvipc::{Event, Mpv, MpvDataType, MpvExt};
use serde_json::json;
use test_log::test;
use tokio::{net::UnixStream, task::JoinHandle};
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

fn test_socket(
    answers: Vec<(bool, String)>,
) -> (UnixStream, JoinHandle<Result<(), LinesCodecError>>) {
    let (socket, server) = UnixStream::pair().unwrap();
    let join_handle = tokio::spawn(async move {
        let mut framed = Framed::new(socket, LinesCodec::new());
        for (unsolicited, answer) in answers {
            if !unsolicited {
                framed.next().await;
            }
            framed.send(answer).await?;
        }
        Ok(())
    });

    (server, join_handle)
}

#[test(tokio::test)]
async fn test_observe_event_successful() {
    let (server, join_handle) = test_socket(vec![
        (
            false,
            json!({ "request_id": 0, "error": "success" }).to_string(),
        ),
        (
            false,
            json!({ "request_id": 0, "error": "success" }).to_string(),
        ),
        (
            true,
            json!({ "data": 64.0, "event": "property-change", "id": 1, "name": "volume" })
                .to_string(),
        ),
    ]);

    let mpv = Mpv::connect_socket(server).await.unwrap();

    mpv.observe_property(1, "volume").await.unwrap();

    let mpv2 = mpv.clone();
    tokio::spawn(async move {
        let event = mpv2.get_event_stream().await.next().await.unwrap().unwrap();

        assert_eq!(
            event,
            Event::PropertyChange {
                id: 1,
                name: "volume".to_string(),
                data: Some(MpvDataType::Double(64.0))
            }
        )
    });

    mpv.set_property("volume", 64.0).await.unwrap();

    join_handle.await.unwrap().unwrap();
}
