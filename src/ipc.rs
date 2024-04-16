use super::*;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::mem;
use tokio::net::UnixStream;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{oneshot, Mutex};
use tokio_util::codec::{Framed, LinesCodec};

pub(crate) struct MpvIpc {
    socket: Framed<UnixStream, LinesCodec>,
    command_channel: Receiver<(MpvIpcCommand, oneshot::Sender<MpvIpcResponse>)>,
    socket_lock: Mutex<()>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MpvIpcCommand {
    Command(Vec<String>),
    GetProperty(String),
    SetProperty(String, Value),
    ObserveProperty(isize, String),
    UnobserveProperty(isize),
    Exit,
}

#[derive(Debug, Clone)]
pub(crate) struct MpvIpcResponse(pub(crate) Result<Option<Value>, Error>);

impl MpvIpc {
    pub(crate) fn new(
        socket: UnixStream,
        command_channel: Receiver<(MpvIpcCommand, oneshot::Sender<MpvIpcResponse>)>,
    ) -> Self {
        MpvIpc {
            socket: Framed::new(socket, LinesCodec::new()),
            command_channel,
            socket_lock: Mutex::new(()),
        }
    }

    pub(crate) async fn send_command(&mut self, command: &[&str]) -> Result<Option<Value>, Error> {
        let lock = self.socket_lock.lock().await;
        // START CRITICAL SECTION
        let ipc_command = json!({ "command": command });
        let ipc_command_str = serde_json::to_string(&ipc_command)
            .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())))?;

        log::trace!("Sending command: {}", ipc_command_str);

        self.socket
            .send(ipc_command_str)
            .await
            .map_err(|why| Error(ErrorCode::ConnectError(why.to_string())))?;

        let response = self
            .socket
            .next()
            .await
            .ok_or(Error(ErrorCode::MissingValue))?
            .map_err(|why| Error(ErrorCode::ConnectError(why.to_string())))?;

        // END CRITICAL SECTION
        mem::drop(lock);

        log::trace!("Received response: {}", response);

        serde_json::from_str::<Value>(&response)
            .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())))
            .and_then(parse_mpv_response_data)

    }

    pub(crate) async fn get_mpv_property(&mut self, property: &str) -> Result<Option<Value>, Error> {
        self.send_command(&["get_property", property]).await
    }

    pub(crate) async fn set_mpv_property(
        &mut self,
        property: &str,
        value: Value,
    ) -> Result<Option<Value>, Error> {
        let str_value = match &value {
          Value::String(s) => s,
          v => &serde_json::to_string(&v).unwrap()
        };
        self.send_command(&["set_property", property, &str_value])
            .await
    }

    pub(crate) async fn observe_property(
        &mut self,
        id: isize,
        property: &str,
    ) -> Result<Option<Value>, Error> {
        self.send_command(&["observe_property", &id.to_string(), property])
            .await
    }

    pub(crate) async fn unobserve_property(&mut self, id: isize) -> Result<Option<Value>, Error> {
        self.send_command(&["unobserve_property", &id.to_string()])
            .await
    }

    pub(crate) async fn run(mut self) -> Result<(), Error> {
        loop {
            tokio::select! {
              Some(event) = self.socket.next() => {
                log::trace!("Handling event: {:?}", serde_json::from_str::<Value>(&event.unwrap()).unwrap());
                // TODO: handle event
              }
              Some((cmd, tx)) = self.command_channel.recv() => {
                  log::trace!("Handling command: {:?}", cmd);
                  match cmd {
                      MpvIpcCommand::Command(command) => {
                          let refs = command.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
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
    result.map(|opt| opt.map(|val| val.clone()))
}