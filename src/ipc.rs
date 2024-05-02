//! IPC handling thread/task. Handles communication between [`Mpv`](crate::Mpv) instances and mpv's unix socket

use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::{
    net::UnixStream,
    sync::{broadcast, mpsc, oneshot},
};
use tokio_util::codec::{Framed, LinesCodec};

use crate::{Error, ErrorCode};

/// Container for all state that regards communication with the mpv IPC socket
/// and message passing with [`Mpv`](crate::Mpv) controllers.
pub(crate) struct MpvIpc {
    socket: Framed<UnixStream, LinesCodec>,
    command_channel: mpsc::Receiver<(MpvIpcCommand, oneshot::Sender<MpvIpcResponse>)>,
    event_channel: broadcast::Sender<MpvIpcEvent>,
}

/// Commands that can be sent to [`MpvIpc`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MpvIpcCommand {
    Command(Vec<String>),
    GetProperty(String),
    SetProperty(String, Value),
    ObserveProperty(isize, String),
    UnobserveProperty(isize),
    Exit,
}

/// [`MpvIpc`]'s response to a [`MpvIpcCommand`].
#[derive(Debug, Clone)]
pub(crate) struct MpvIpcResponse(pub(crate) Result<Option<Value>, Error>);

/// A deserialized and partially parsed event from mpv.
#[derive(Debug, Clone)]
pub(crate) struct MpvIpcEvent(pub(crate) Value);

impl MpvIpc {
    pub(crate) fn new(
        socket: UnixStream,
        command_channel: mpsc::Receiver<(MpvIpcCommand, oneshot::Sender<MpvIpcResponse>)>,
        event_channel: broadcast::Sender<MpvIpcEvent>,
    ) -> Self {
        MpvIpc {
            socket: Framed::new(socket, LinesCodec::new()),
            command_channel,
            event_channel,
        }
    }

    pub(crate) async fn send_command(&mut self, command: &[Value]) -> Result<Option<Value>, Error> {
        let ipc_command = json!({ "command": command });
        let ipc_command_str = serde_json::to_string(&ipc_command)
            .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())))?;

        log::trace!("Sending command: {}", ipc_command_str);

        self.socket
            .send(ipc_command_str)
            .await
            .map_err(|why| Error(ErrorCode::ConnectError(why.to_string())))?;

        let response = loop {
            let response = self
                .socket
                .next()
                .await
                .ok_or(Error(ErrorCode::MissingValue))?
                .map_err(|why| Error(ErrorCode::ConnectError(why.to_string())))?;

            let parsed_response = serde_json::from_str::<Value>(&response)
                .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())));

            if parsed_response
                .as_ref()
                .ok()
                .and_then(|v| v.as_object().map(|o| o.contains_key("event")))
                .unwrap_or(false)
            {
                self.handle_event(parsed_response).await;
            } else {
                break parsed_response;
            }
        };

        log::trace!("Received response: {:?}", response);

        parse_mpv_response_data(response?)
    }

    pub(crate) async fn get_mpv_property(
        &mut self,
        property: &str,
    ) -> Result<Option<Value>, Error> {
        self.send_command(&[json!("get_property"), json!(property)])
            .await
    }

    pub(crate) async fn set_mpv_property(
        &mut self,
        property: &str,
        value: Value,
    ) -> Result<Option<Value>, Error> {
        self.send_command(&[json!("set_property"), json!(property), value])
            .await
    }

    pub(crate) async fn observe_property(
        &mut self,
        id: isize,
        property: &str,
    ) -> Result<Option<Value>, Error> {
        self.send_command(&[json!("observe_property"), json!(id), json!(property)])
            .await
    }

    pub(crate) async fn unobserve_property(&mut self, id: isize) -> Result<Option<Value>, Error> {
        self.send_command(&[json!("unobserve_property"), json!(id)])
            .await
    }

    async fn handle_event(&mut self, event: Result<Value, Error>) {
        match &event {
            Ok(event) => {
                log::trace!("Parsed event: {:?}", event);
                if let Err(broadcast::error::SendError(_)) =
                    self.event_channel.send(MpvIpcEvent(event.to_owned()))
                {
                    log::trace!("Failed to send event to channel, ignoring");
                }
            }
            Err(e) => {
                log::trace!("Error parsing event, ignoring:\n  {:?}\n  {:?}", &event, e);
            }
        }
    }

    pub(crate) async fn run(mut self) -> Result<(), Error> {
        loop {
            tokio::select! {
              Some(event) = self.socket.next() => {
                log::trace!("Got event: {:?}", event);

                let parsed_event = event
                    .map_err(|why| Error(ErrorCode::ConnectError(why.to_string())))
                    .and_then(|event|
                        serde_json::from_str::<Value>(&event)
                        .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string()))));

                self.handle_event(parsed_event).await;
              }
              Some((cmd, tx)) = self.command_channel.recv() => {
                  log::trace!("Handling command: {:?}", cmd);
                  match cmd {
                      MpvIpcCommand::Command(command) => {
                          let refs = command.iter().map(|s| json!(s)).collect::<Vec<Value>>();
                          let response = self.send_command(refs.as_slice()).await;
                          tx.send(MpvIpcResponse(response)).unwrap()
                      }
                      MpvIpcCommand::GetProperty(property) => {
                          let response = self.get_mpv_property(&property).await;
                          tx.send(MpvIpcResponse(response)).unwrap()
                      }
                      MpvIpcCommand::SetProperty(property, value) => {
                          let response = self.set_mpv_property(&property, value).await;
                          tx.send(MpvIpcResponse(response)).unwrap()
                      }
                      MpvIpcCommand::ObserveProperty(id, property) => {
                          let response = self.observe_property(id, &property).await;
                          tx.send(MpvIpcResponse(response)).unwrap()
                      }
                      MpvIpcCommand::UnobserveProperty(id) => {
                          let response = self.unobserve_property(id).await;
                          tx.send(MpvIpcResponse(response)).unwrap()
                      }
                      MpvIpcCommand::Exit => {
                        tx.send(MpvIpcResponse(Ok(None))).unwrap();
                        return Ok(());
                      }
                  }
              }
            }
        }
    }
}

/// This function does the most basic JSON parsing and error handling
/// for status codes and errors that all responses from mpv are
/// expected to contain.
fn parse_mpv_response_data(value: Value) -> Result<Option<Value>, Error> {
    log::trace!("Parsing mpv response data: {:?}", value);
    let result = value
        .as_object()
        .map(|o| (o.get("error").and_then(|e| e.as_str()), o.get("data")))
        .ok_or(Error(ErrorCode::UnexpectedValue))
        .and_then(|(error, data)| match error {
            Some("success") => Ok(data),
            Some(e) => Err(Error(ErrorCode::MpvError(e.to_string()))),
            None => Err(Error(ErrorCode::UnexpectedValue)),
        });
    match &result {
        Ok(v) => log::trace!("Successfully parsed mpv response data: {:?}", v),
        Err(e) => log::trace!("Error parsing mpv response data: {:?}", e),
    }
    result.map(|opt| opt.cloned())
}
