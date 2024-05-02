//! The core API for interacting with [`Mpv`].

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt};
use tokio::{
    net::UnixStream,
    sync::{broadcast, mpsc, oneshot},
};

use crate::{
    ipc::{MpvIpc, MpvIpcCommand, MpvIpcEvent, MpvIpcResponse},
    message_parser::TypeHandler,
    Error, ErrorCode, Event,
};

/// All possible commands that can be sent to mpv.
///
/// Not all commands are guaranteed to be implemented.
/// If something is missing, please open an issue.
///
/// You can also use the `run_command_raw` function to run commands
/// that are not implemented here.
///
/// See <https://mpv.io/manual/master/#list-of-input-commands> for
/// the upstream list of commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MpvCommand {
    LoadFile {
        file: String,
        option: PlaylistAddOptions,
    },
    LoadList {
        file: String,
        option: PlaylistAddOptions,
    },
    PlaylistClear,
    PlaylistMove {
        from: usize,
        to: usize,
    },
    Observe {
        id: isize,
        property: String,
    },
    PlaylistNext,
    PlaylistPrev,
    PlaylistRemove(usize),
    PlaylistShuffle,
    Quit,
    ScriptMessage(Vec<String>),
    ScriptMessageTo {
        target: String,
        args: Vec<String>,
    },
    Seek {
        seconds: f64,
        option: SeekOptions,
    },
    Stop,
    Unobserve(isize),
}

/// Helper trait to keep track of the string literals that mpv expects.
pub(crate) trait IntoRawCommandPart {
    fn into_raw_command_part(self) -> String;
}

/// Generic data type representing all possible data types that mpv can return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MpvDataType {
    Array(Vec<MpvDataType>),
    Bool(bool),
    Double(f64),
    HashMap(HashMap<String, MpvDataType>),
    Null,
    MinusOne,
    Playlist(Playlist),
    String(String),
    Usize(usize),
}

/// A mpv playlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist(pub Vec<PlaylistEntry>);

/// A single entry in the mpv playlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub id: usize,
    pub filename: String,
    pub title: String,
    pub current: bool,
}

/// Options for [`MpvCommand::LoadFile`] and [`MpvCommand::LoadList`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlaylistAddOptions {
    Replace,
    Append,
}

impl IntoRawCommandPart for PlaylistAddOptions {
    fn into_raw_command_part(self) -> String {
        match self {
            PlaylistAddOptions::Replace => "replace".to_string(),
            PlaylistAddOptions::Append => "append".to_string(),
        }
    }
}

/// Options for [`MpvCommand::Seek`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SeekOptions {
    Relative,
    Absolute,
    RelativePercent,
    AbsolutePercent,
}

impl IntoRawCommandPart for SeekOptions {
    fn into_raw_command_part(self) -> String {
        match self {
            SeekOptions::Relative => "relative".to_string(),
            SeekOptions::Absolute => "absolute".to_string(),
            SeekOptions::RelativePercent => "relative-percent".to_string(),
            SeekOptions::AbsolutePercent => "absolute-percent".to_string(),
        }
    }
}

/// A trait for specifying how to extract and parse a value returned through [`Mpv::get_property`].
pub trait GetPropertyTypeHandler: Sized {
    // TODO: fix this
    #[allow(async_fn_in_trait)]
    async fn get_property_generic(instance: &Mpv, property: &str) -> Result<Self, Error>;
}

impl<T> GetPropertyTypeHandler for T
where
    T: TypeHandler,
{
    async fn get_property_generic(instance: &Mpv, property: &str) -> Result<T, Error> {
        instance
            .get_property_value(property)
            .await
            .and_then(T::get_value)
    }
}

/// A trait for specifying how to serialize and set a value through [`Mpv::set_property`].
pub trait SetPropertyTypeHandler<T> {
    // TODO: fix this
    #[allow(async_fn_in_trait)]
    async fn set_property_generic(instance: &Mpv, property: &str, value: T) -> Result<(), Error>;
}

impl<T> SetPropertyTypeHandler<T> for T
where
    T: Serialize,
{
    async fn set_property_generic(instance: &Mpv, property: &str, value: T) -> Result<(), Error> {
        let (res_tx, res_rx) = oneshot::channel();
        let value = serde_json::to_value(value)
            .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())))?;
        instance
            .command_sender
            .send((
                MpvIpcCommand::SetProperty(property.to_owned(), value),
                res_tx,
            ))
            .await
            .map_err(|_| {
                Error(ErrorCode::ConnectError(
                    "Failed to send command".to_string(),
                ))
            })?;

        match res_rx.await {
            Ok(MpvIpcResponse(response)) => response.map(|_| ()),
            Err(err) => Err(Error(ErrorCode::ConnectError(err.to_string()))),
        }
    }
}

/// The main struct for interacting with mpv.
///
/// This struct provides the core API for interacting with mpv.
/// These functions are the building blocks for the higher-level API provided by the `MpvExt` trait.
/// They can also be used directly to interact with mpv in a more flexible way, mostly returning JSON values.
///
/// The `Mpv` struct can be cloned freely, and shared anywhere.
/// It only contains a message passing channel to the tokio task that handles the IPC communication with mpv.
#[derive(Clone)]
pub struct Mpv {
    command_sender: mpsc::Sender<(MpvIpcCommand, oneshot::Sender<MpvIpcResponse>)>,
    broadcast_channel: broadcast::Sender<MpvIpcEvent>,
}

impl fmt::Debug for Mpv {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Mpv").finish()
    }
}

impl Mpv {
    pub async fn connect(socket_path: &str) -> Result<Mpv, Error> {
        log::debug!("Connecting to mpv socket at {}", socket_path);

        let socket = match UnixStream::connect(socket_path).await {
            Ok(stream) => Ok(stream),
            Err(internal_error) => Err(Error(ErrorCode::ConnectError(internal_error.to_string()))),
        }?;

        Self::connect_socket(socket).await
    }

    pub async fn connect_socket(socket: UnixStream) -> Result<Mpv, Error> {
        let (com_tx, com_rx) = mpsc::channel(100);
        let (ev_tx, _) = broadcast::channel(100);
        let ipc = MpvIpc::new(socket, com_rx, ev_tx.clone());

        log::debug!("Starting IPC handler");
        tokio::spawn(ipc.run());

        Ok(Mpv {
            command_sender: com_tx,
            broadcast_channel: ev_tx,
        })
    }

    pub async fn disconnect(&self) -> Result<(), Error> {
        let (res_tx, res_rx) = oneshot::channel();
        self.command_sender
            .send((MpvIpcCommand::Exit, res_tx))
            .await
            .map_err(|_| {
                Error(ErrorCode::ConnectError(
                    "Failed to send command".to_string(),
                ))
            })?;
        match res_rx.await {
            Ok(MpvIpcResponse(response)) => response.map(|_| ()),
            Err(err) => Err(Error(ErrorCode::ConnectError(err.to_string()))),
        }
    }

    pub async fn get_event_stream(&self) -> impl futures::Stream<Item = Result<Event, Error>> {
        tokio_stream::wrappers::BroadcastStream::new(self.broadcast_channel.subscribe()).map(
            |event| match event {
                Ok(event) => crate::event_parser::parse_event(event),
                Err(_) => Err(Error(ErrorCode::ConnectError(
                    "Failed to receive event".to_string(),
                ))),
            },
        )
    }

    /// Run a custom command.
    /// This should only be used if the desired command is not implemented
    /// with [MpvCommand].
    pub async fn run_command_raw(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<Option<Value>, Error> {
        let command = Vec::from(
            [command]
                .iter()
                .chain(args.iter())
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .as_slice(),
        );
        let (res_tx, res_rx) = oneshot::channel();
        self.command_sender
            .send((MpvIpcCommand::Command(command), res_tx))
            .await
            .map_err(|_| {
                Error(ErrorCode::ConnectError(
                    "Failed to send command".to_string(),
                ))
            })?;

        match res_rx.await {
            Ok(MpvIpcResponse(response)) => response,
            Err(err) => Err(Error(ErrorCode::ConnectError(err.to_string()))),
        }
    }

    async fn run_command_raw_ignore_value(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<(), Error> {
        self.run_command_raw(command, args).await.map(|_| ())
    }

    /// # Description
    ///
    /// Runs mpv commands. The arguments are passed as a String-Vector reference:
    ///
    /// ## Input arguments
    ///
    /// - **command**   defines the mpv command that should be executed
    /// - **args**      a slice of `&str`'s which define the arguments
    ///
    /// # Example
    /// ```
    /// use mpvipc::{Mpv, Error};
    /// fn main() -> Result<(), Error> {
    ///     let mpv = Mpv::connect("/tmp/mpvsocket")?;
    ///
    ///     //Run command 'playlist-shuffle' which takes no arguments
    ///     mpv.run_command(MpvCommand::PlaylistShuffle)?;
    ///
    ///     //Run command 'seek' which in this case takes two arguments
    ///     mpv.run_command(MpvCommand::Seek {
    ///         seconds: 0f64,
    ///         option: SeekOptions::Absolute,
    ///     })?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_command(&self, command: MpvCommand) -> Result<(), Error> {
        log::trace!("Running command: {:?}", command);
        let result = match command {
            MpvCommand::LoadFile { file, option } => {
                self.run_command_raw_ignore_value(
                    "loadfile",
                    &[file.as_ref(), option.into_raw_command_part().as_str()],
                )
                .await
            }
            MpvCommand::LoadList { file, option } => {
                self.run_command_raw_ignore_value(
                    "loadlist",
                    &[file.as_ref(), option.into_raw_command_part().as_str()],
                )
                .await
            }
            MpvCommand::Observe { id, property } => {
                let (res_tx, res_rx) = oneshot::channel();
                self.command_sender
                    .send((MpvIpcCommand::ObserveProperty(id, property), res_tx))
                    .await
                    .map_err(|_| {
                        Error(ErrorCode::ConnectError(
                            "Failed to send command".to_string(),
                        ))
                    })?;

                match res_rx.await {
                    Ok(MpvIpcResponse(response)) => response.map(|_| ()),
                    Err(err) => Err(Error(ErrorCode::ConnectError(err.to_string()))),
                }
            }
            MpvCommand::PlaylistClear => {
                self.run_command_raw_ignore_value("playlist-clear", &[])
                    .await
            }
            MpvCommand::PlaylistMove { from, to } => {
                self.run_command_raw_ignore_value(
                    "playlist-move",
                    &[&from.to_string(), &to.to_string()],
                )
                .await
            }
            MpvCommand::PlaylistNext => {
                self.run_command_raw_ignore_value("playlist-next", &[])
                    .await
            }
            MpvCommand::PlaylistPrev => {
                self.run_command_raw_ignore_value("playlist-prev", &[])
                    .await
            }
            MpvCommand::PlaylistRemove(id) => {
                self.run_command_raw_ignore_value("playlist-remove", &[&id.to_string()])
                    .await
            }
            MpvCommand::PlaylistShuffle => {
                self.run_command_raw_ignore_value("playlist-shuffle", &[])
                    .await
            }
            MpvCommand::Quit => self.run_command_raw_ignore_value("quit", &[]).await,
            MpvCommand::ScriptMessage(args) => {
                let str_args: Vec<_> = args.iter().map(String::as_str).collect();
                self.run_command_raw_ignore_value("script-message", &str_args)
                    .await
            }
            MpvCommand::ScriptMessageTo { target, args } => {
                let mut cmd_args: Vec<_> = vec![target.as_str()];
                let mut str_args: Vec<_> = args.iter().map(String::as_str).collect();
                cmd_args.append(&mut str_args);
                self.run_command_raw_ignore_value("script-message-to", &cmd_args)
                    .await
            }
            MpvCommand::Seek { seconds, option } => {
                self.run_command_raw_ignore_value(
                    "seek",
                    &[
                        &seconds.to_string(),
                        option.into_raw_command_part().as_str(),
                    ],
                )
                .await
            }
            MpvCommand::Stop => self.run_command_raw_ignore_value("stop", &[]).await,
            MpvCommand::Unobserve(id) => {
                let (res_tx, res_rx) = oneshot::channel();
                self.command_sender
                    .send((MpvIpcCommand::UnobserveProperty(id), res_tx))
                    .await
                    .unwrap();
                match res_rx.await {
                    Ok(MpvIpcResponse(response)) => response.map(|_| ()),
                    Err(err) => Err(Error(ErrorCode::ConnectError(err.to_string()))),
                }
            }
        };
        log::trace!("Command result: {:?}", result);
        result
    }

    /// # Description
    ///
    /// Retrieves the property value from mpv.
    ///
    /// ## Supported types
    /// - `String`
    /// - `bool`
    /// - `HashMap<String, String>` (e.g. for the 'metadata' property)
    /// - `Vec<PlaylistEntry>` (for the 'playlist' property)
    /// - `usize`
    /// - `f64`
    ///
    /// ## Input arguments
    ///
    /// - **property** defines the mpv property that should be retrieved
    ///
    /// # Example
    /// ```
    /// use mpvipc::{Mpv, Error};
    /// async fn main() -> Result<(), Error> {
    ///     let mpv = Mpv::connect("/tmp/mpvsocket")?;
    ///     let paused: bool = mpv.get_property("pause").await?;
    ///     let title: String = mpv.get_property("media-title").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_property<T: GetPropertyTypeHandler>(
        &self,
        property: &str,
    ) -> Result<T, Error> {
        T::get_property_generic(self, property).await
    }

    /// # Description
    ///
    /// Retrieves the property value from mpv.
    /// The result is always of type String, regardless of the type of the value of the mpv property
    ///
    /// ## Input arguments
    ///
    /// - **property** defines the mpv property that should be retrieved
    ///
    /// # Example
    ///
    /// ```
    /// use mpvipc::{Mpv, Error};
    /// fn main() -> Result<(), Error> {
    ///     let mpv = Mpv::connect("/tmp/mpvsocket")?;
    ///     let title = mpv.get_property_string("media-title")?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_property_value(&self, property: &str) -> Result<Value, Error> {
        let (res_tx, res_rx) = oneshot::channel();
        self.command_sender
            .send((MpvIpcCommand::GetProperty(property.to_owned()), res_tx))
            .await
            .map_err(|_| {
                Error(ErrorCode::ConnectError(
                    "Failed to send command".to_string(),
                ))
            })?;
        match res_rx.await {
            Ok(MpvIpcResponse(response)) => {
                response.and_then(|value| value.ok_or(Error(ErrorCode::MissingValue)))
            }
            Err(err) => Err(Error(ErrorCode::ConnectError(err.to_string()))),
        }
    }

    /// # Description
    ///
    /// Sets the mpv property _`<property>`_ to _`<value>`_.
    ///
    /// ## Supported types
    /// - `String`
    /// - `bool`
    /// - `f64`
    /// - `usize`
    ///
    /// ## Input arguments
    ///
    /// - **property** defines the mpv property that should be retrieved
    /// - **value** defines the value of the given mpv property _`<property>`_
    ///
    /// # Example
    /// ```
    /// use mpvipc::{Mpv, Error};
    /// fn async main() -> Result<(), Error> {
    ///     let mpv = Mpv::connect("/tmp/mpvsocket")?;
    ///     mpv.set_property("pause", true).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_property<T: SetPropertyTypeHandler<T>>(
        &self,
        property: &str,
        value: T,
    ) -> Result<(), Error> {
        T::set_property_generic(self, property, value).await
    }
}
