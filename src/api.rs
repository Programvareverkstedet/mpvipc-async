use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{self, Display},
};
use tokio::{net::UnixStream, sync::oneshot};

use crate::ipc::{MpvIpc, MpvIpcCommand, MpvIpcResponse};
use crate::message_parser::TypeHandler;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Shutdown,
    StartFile,
    EndFile,
    FileLoaded,
    TracksChanged,
    TrackSwitched,
    Idle,
    Pause,
    Unpause,
    Tick,
    VideoReconfig,
    AudioReconfig,
    MetadataUpdate,
    Seek,
    PlaybackRestart,
    PropertyChange { id: usize, property: Property },
    ChapterChange,
    ClientMessage { args: Vec<String> },
    Unimplemented,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Property {
    Path(Option<String>),
    Pause(bool),
    PlaybackTime(Option<f64>),
    Duration(Option<f64>),
    Metadata(Option<HashMap<String, MpvDataType>>),
    Unknown { name: String, data: MpvDataType },
}

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

trait IntoRawCommandPart {
    fn into_raw_command_part(self) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MpvDataType {
    Array(Vec<MpvDataType>),
    Bool(bool),
    Double(f64),
    HashMap(HashMap<String, MpvDataType>),
    Null,
    Playlist(Playlist),
    String(String),
    Usize(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NumberChangeOptions {
    Absolute,
    Increase,
    Decrease,
}

impl IntoRawCommandPart for NumberChangeOptions {
    fn into_raw_command_part(self) -> String {
        match self {
            NumberChangeOptions::Absolute => "absolute".to_string(),
            NumberChangeOptions::Increase => "increase".to_string(),
            NumberChangeOptions::Decrease => "decrease".to_string(),
        }
    }
}

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlaylistAddTypeOptions {
    File,
    Playlist,
}

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Switch {
    On,
    Off,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    MpvError(String),
    JsonParseError(String),
    ConnectError(String),
    JsonContainsUnexptectedType,
    UnexpectedResult,
    UnexpectedValue,
    MissingValue,
    UnsupportedType,
    ValueDoesNotContainBool,
    ValueDoesNotContainF64,
    ValueDoesNotContainHashMap,
    ValueDoesNotContainPlaylist,
    ValueDoesNotContainString,
    ValueDoesNotContainUsize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub id: usize,
    pub filename: String,
    pub title: String,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist(pub Vec<PlaylistEntry>);

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Error(pub ErrorCode);

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorCode::ConnectError(ref msg) => f.write_str(&format!("ConnectError: {}", msg)),
            ErrorCode::JsonParseError(ref msg) => f.write_str(&format!("JsonParseError: {}", msg)),
            ErrorCode::MpvError(ref msg) => f.write_str(&format!("MpvError: {}", msg)),
            ErrorCode::JsonContainsUnexptectedType => {
                f.write_str("Mpv sent a value with an unexpected type")
            }
            ErrorCode::UnexpectedResult => f.write_str("Unexpected result received"),
            ErrorCode::UnexpectedValue => f.write_str("Unexpected value received"),
            ErrorCode::MissingValue => f.write_str("Missing value"),
            ErrorCode::UnsupportedType => f.write_str("Unsupported type received"),
            ErrorCode::ValueDoesNotContainBool => {
                f.write_str("The received value is not of type \'std::bool\'")
            }
            ErrorCode::ValueDoesNotContainF64 => {
                f.write_str("The received value is not of type \'std::f64\'")
            }
            ErrorCode::ValueDoesNotContainHashMap => {
                f.write_str("The received value is not of type \'std::collections::HashMap\'")
            }
            ErrorCode::ValueDoesNotContainPlaylist => {
                f.write_str("The received value is not of type \'mpvipc::Playlist\'")
            }
            ErrorCode::ValueDoesNotContainString => {
                f.write_str("The received value is not of type \'std::string::String\'")
            }
            ErrorCode::ValueDoesNotContainUsize => {
                f.write_str("The received value is not of type \'std::usize\'")
            }
        }
    }
}

#[derive(Clone)]
pub struct Mpv {
    command_sender: tokio::sync::mpsc::Sender<(MpvIpcCommand, oneshot::Sender<MpvIpcResponse>)>,
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
        let (com_tx, com_rx) = tokio::sync::mpsc::channel(100);
        let ipc = MpvIpc::new(socket, com_rx);

        log::debug!("Starting IPC handler");
        tokio::spawn(ipc.run());

        Ok(Mpv {
            command_sender: com_tx,
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

    // pub fn get_stream_ref(&self) -> &UnixStream {
    //     &self.stream
    // }

    pub async fn get_metadata(&self) -> Result<HashMap<String, MpvDataType>, Error> {
        self.get_property("metadata").await
    }

    pub async fn get_playlist(&self) -> Result<Playlist, Error> {
        self.get_property::<Vec<PlaylistEntry>>("playlist")
            .await
            .map(|entries| Playlist(entries))
    }

    /// # Description
    ///
    /// Retrieves the property value from mpv.
    ///
    /// ## Supported types
    /// - String
    /// - bool
    /// - HashMap<String, String> (e.g. for the 'metadata' property)
    /// - Vec<PlaylistEntry> (for the 'playlist' property)
    /// - usize
    /// - f64
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
            Ok(MpvIpcResponse(response)) => response.and_then(|value| {
                value.ok_or(Error(ErrorCode::MissingValue))
            }),
            Err(err) => Err(Error(ErrorCode::ConnectError(err.to_string()))),
        }
    }

    pub async fn kill(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::Quit).await
    }

    /// # Description
    ///
    /// Waits until an mpv event occurs and returns the Event.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut mpv = Mpv::connect("/tmp/mpvsocket")?;
    /// loop {
    ///     let event = mpv.event_listen()?;
    ///     println!("{:?}", event);
    /// }
    /// ```
    // pub fn event_listen(&mut self) -> Result<Event, Error> {
    //     listen(self)
    // }

    // pub fn event_listen_raw(&mut self) -> String {
    //     listen_raw(self)
    // }

    pub async fn next(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistNext).await
    }

    pub async fn observe_property(&self, id: isize, property: &str) -> Result<(), Error> {
        self.run_command(MpvCommand::Observe {
            id,
            property: property.to_string(),
        })
        .await
    }

    pub async fn unobserve_property(&self, id: isize) -> Result<(), Error> {
        self.run_command(MpvCommand::Unobserve(id)).await
    }

    pub async fn pause(&self) -> Result<(), Error> {
        self.set_property("pause", true).await
    }

    pub async fn prev(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistPrev).await
    }

    pub async fn restart(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::Seek {
            seconds: 0f64,
            option: SeekOptions::Absolute,
        })
        .await
    }

    /// # Description
    ///
    /// Runs mpv commands. The arguments are passed as a String-Vector reference:
    ///
    /// ## Input arguments
    ///
    /// - **command**   defines the mpv command that should be executed
    /// - **args**      a slice of &str's which define the arguments
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

    /// Run a custom command.
    /// This should only be used if the desired command is not implemented
    /// with [MpvCommand].
    pub async fn run_command_raw(&self, command: &str, args: &[&str]) -> Result<Option<Value>, Error> {
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

    pub async fn playlist_add(
        &self,
        file: &str,
        file_type: PlaylistAddTypeOptions,
        option: PlaylistAddOptions,
    ) -> Result<(), Error> {
        match file_type {
            PlaylistAddTypeOptions::File => {
                self.run_command(MpvCommand::LoadFile {
                    file: file.to_string(),
                    option,
                })
                .await
            }

            PlaylistAddTypeOptions::Playlist => {
                self.run_command(MpvCommand::LoadList {
                    file: file.to_string(),
                    option,
                })
                .await
            }
        }
    }

    pub async fn playlist_clear(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistClear).await
    }

    pub async fn playlist_move_id(&self, from: usize, to: usize) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistMove { from, to })
            .await
    }

    pub async fn playlist_play_id(&self, id: usize) -> Result<(), Error> {
        self.set_property("playlist-pos", id).await
    }

    pub async fn playlist_play_next(&self, id: usize) -> Result<(), Error> {
        match self.get_property::<usize>("playlist-pos").await {
            Ok(current_id) => {
                self.run_command(MpvCommand::PlaylistMove {
                    from: id,
                    to: current_id + 1,
                })
                .await
            }
            Err(msg) => Err(msg),
        }
    }

    pub async fn playlist_remove_id(&self, id: usize) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistRemove(id)).await
    }

    pub async fn playlist_shuffle(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistShuffle).await
    }

    pub async fn seek(&self, seconds: f64, option: SeekOptions) -> Result<(), Error> {
        self.run_command(MpvCommand::Seek { seconds, option }).await
    }

    pub async fn set_loop_file(&self, option: Switch) -> Result<(), Error> {
        let enabled = match option {
            Switch::On => "inf",
            Switch::Off => "no",
            Switch::Toggle => {
                self.get_property::<String>("loop-file")
                    .await
                    .map(|s| match s.as_str() {
                        "inf" => "no",
                        "no" => "inf",
                        _ => "no",
                    })?
            }
        };
        self.set_property("loop-file", enabled).await
    }

    pub async fn set_loop_playlist(&self, option: Switch) -> Result<(), Error> {
        let enabled = match option {
            Switch::On => "inf",
            Switch::Off => "no",
            Switch::Toggle => {
                self.get_property::<String>("loop-playlist")
                    .await
                    .map(|s| match s.as_str() {
                        "inf" => "no",
                        "no" => "inf",
                        _ => "no",
                    })?
            }
        };
        self.set_property("loo-playlist", enabled).await
    }

    pub async fn set_mute(&self, option: Switch) -> Result<(), Error> {
        let enabled = match option {
            Switch::On => "yes",
            Switch::Off => "no",
            Switch::Toggle => {
                self.get_property::<String>("mute")
                    .await
                    .map(|s| match s.as_str() {
                        "yes" => "no",
                        "no" => "yes",
                        _ => "no",
                    })?
            }
        };
        self.set_property("mute", enabled).await
    }

    /// # Description
    ///
    /// Sets the mpv property _<property>_ to _<value>_.
    ///
    /// ## Supported types
    /// - String
    /// - bool
    /// - f64
    /// - usize
    ///
    /// ## Input arguments
    ///
    /// - **property** defines the mpv property that should be retrieved
    /// - **value** defines the value of the given mpv property _<property>_
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

    pub async fn set_speed(
        &self,
        input_speed: f64,
        option: NumberChangeOptions,
    ) -> Result<(), Error> {
        match self.get_property::<f64>("speed").await {
            Ok(speed) => match option {
                NumberChangeOptions::Increase => {
                    self.set_property("speed", speed + input_speed).await
                }

                NumberChangeOptions::Decrease => {
                    self.set_property("speed", speed - input_speed).await
                }

                NumberChangeOptions::Absolute => self.set_property("speed", input_speed).await,
            },
            Err(msg) => Err(msg),
        }
    }

    pub async fn set_volume(
        &self,
        input_volume: f64,
        option: NumberChangeOptions,
    ) -> Result<(), Error> {
        match self.get_property::<f64>("volume").await {
            Ok(volume) => match option {
                NumberChangeOptions::Increase => {
                    self.set_property("volume", volume + input_volume).await
                }

                NumberChangeOptions::Decrease => {
                    self.set_property("volume", volume - input_volume).await
                }

                NumberChangeOptions::Absolute => self.set_property("volume", input_volume).await,
            },
            Err(msg) => Err(msg),
        }
    }

    pub async fn stop(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::Stop).await
    }

    pub async fn toggle(&self) -> Result<(), Error> {
        self.run_command_raw("cycle", &["pause"]).await.map(|_| ())
    }
}
