//! High-level API extension for [`Mpv`].

use crate::{
    IntoRawCommandPart, Mpv, MpvCommand, MpvDataType, MpvError, Playlist, PlaylistAddOptions,
    PlaylistEntry, SeekOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic high-level command for changing a number property.
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

/// Generic high-level switch for toggling boolean properties.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Switch {
    On,
    Off,
    Toggle,
}

/// Options for [`MpvExt::playlist_add`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlaylistAddTypeOptions {
    File,
    Playlist,
}

/// A set of typesafe high-level functions to interact with [`Mpv`].
// TODO: fix this
#[allow(async_fn_in_trait)]
pub trait MpvExt {
    /// Stop the player completely (as opposed to pausing),
    /// removing the pointer to the current video.
    async fn stop(&self) -> Result<(), MpvError>;

    /// Set the volume of the player.
    async fn set_volume(
        &self,
        input_volume: f64,
        option: NumberChangeOptions,
    ) -> Result<(), MpvError>;

    /// Set the playback speed of the player.
    async fn set_speed(
        &self,
        input_speed: f64,
        option: NumberChangeOptions,
    ) -> Result<(), MpvError>;

    /// Toggle/set the pause state of the player.
    async fn set_playback(&self, option: Switch) -> Result<(), MpvError>;

    /// Toggle/set the mute state of the player.
    async fn set_mute(&self, option: Switch) -> Result<(), MpvError>;

    /// Toggle/set whether the player should loop the current playlist.
    async fn set_loop_playlist(&self, option: Switch) -> Result<(), MpvError>;

    /// Toggle/set whether the player should loop the current video.
    async fn set_loop_file(&self, option: Switch) -> Result<(), MpvError>;

    /// Seek to a specific position in the current video.
    async fn seek(&self, seconds: f64, option: SeekOptions) -> Result<(), MpvError>;

    /// Shuffle the current playlist.
    async fn playlist_shuffle(&self) -> Result<(), MpvError>;

    /// Remove an entry from the playlist.
    async fn playlist_remove_id(&self, id: usize) -> Result<(), MpvError>;

    /// Play the next entry in the playlist.
    async fn playlist_play_next(&self, id: usize) -> Result<(), MpvError>;

    /// Play a specific entry in the playlist.
    async fn playlist_play_id(&self, id: usize) -> Result<(), MpvError>;

    /// Move an entry in the playlist.
    ///
    /// The `from` parameter is the current position of the entry, and the `to` parameter is the new position.
    /// Mpv will then move the entry from the `from` position to the `to` position,
    /// shifting after `to` one number up. Paradoxically, that means that moving an entry further down the list
    /// will result in a final position that is one less than the `to` parameter.
    async fn playlist_move_id(&self, from: usize, to: usize) -> Result<(), MpvError>;

    /// Remove all entries from the playlist.
    async fn playlist_clear(&self) -> Result<(), MpvError>;

    /// Add a file or playlist to the playlist.
    async fn playlist_add(
        &self,
        file: &str,
        file_type: PlaylistAddTypeOptions,
        option: PlaylistAddOptions,
    ) -> Result<(), MpvError>;

    /// Start the current video from the beginning.
    async fn restart(&self) -> Result<(), MpvError>;

    /// Play the previous entry in the playlist.
    async fn prev(&self) -> Result<(), MpvError>;

    /// Notify mpv to send events whenever a property changes.
    /// See [`Mpv::get_event_stream`] and [`Property`](crate::Property) for more information.
    async fn observe_property(&self, id: usize, property: &str) -> Result<(), MpvError>;

    /// Stop observing a property.
    /// See [`Mpv::get_event_stream`] and [`Property`](crate::Property) for more information.
    async fn unobserve_property(&self, id: usize) -> Result<(), MpvError>;

    /// Skip to the next entry in the playlist.
    async fn next(&self) -> Result<(), MpvError>;

    /// Stop mpv completely, and kill the process.
    ///
    /// Note that this is different than forcefully killing the process using
    /// as handle to a subprocess, it will only send a command to mpv to ask
    /// it to exit itself. If mpv is stuck, it may not respond to this command.
    async fn kill(&self) -> Result<(), MpvError>;

    /// Get a list of all entries in the playlist.
    async fn get_playlist(&self) -> Result<Playlist, MpvError>;

    /// Get metadata about the current video.
    async fn get_metadata(&self) -> Result<HashMap<String, MpvDataType>, MpvError>;
}

impl MpvExt for Mpv {
    async fn get_metadata(&self) -> Result<HashMap<String, MpvDataType>, MpvError> {
        self.get_property("metadata").await
    }

    async fn get_playlist(&self) -> Result<Playlist, MpvError> {
        self.get_property::<Vec<PlaylistEntry>>("playlist")
            .await
            .map(Playlist)
    }

    async fn kill(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Quit).await
    }

    async fn next(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistNext).await
    }

    async fn observe_property(&self, id: usize, property: &str) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Observe {
            id,
            property: property.to_string(),
        })
        .await
    }

    async fn unobserve_property(&self, id: usize) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Unobserve(id)).await
    }

    async fn set_playback(&self, option: Switch) -> Result<(), MpvError> {
        let enabled = match option {
            Switch::On => "yes",
            Switch::Off => "no",
            Switch::Toggle => {
                self.get_property::<String>("pause")
                    .await
                    .map(|s| match s.as_str() {
                        "yes" => "no",
                        "no" => "yes",
                        _ => "no",
                    })?
            }
        };
        self.set_property("pause", enabled).await
    }

    async fn prev(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistPrev).await
    }

    async fn restart(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Seek {
            seconds: 0f64,
            option: SeekOptions::Absolute,
        })
        .await
    }

    async fn playlist_add(
        &self,
        file: &str,
        file_type: PlaylistAddTypeOptions,
        option: PlaylistAddOptions,
    ) -> Result<(), MpvError> {
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

    async fn playlist_clear(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistClear).await
    }

    async fn playlist_move_id(&self, from: usize, to: usize) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistMove { from, to })
            .await
    }

    async fn playlist_play_id(&self, id: usize) -> Result<(), MpvError> {
        self.set_property("playlist-pos", id).await
    }

    async fn playlist_play_next(&self, id: usize) -> Result<(), MpvError> {
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

    async fn playlist_remove_id(&self, id: usize) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistRemove(id)).await
    }

    async fn playlist_shuffle(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistShuffle).await
    }

    async fn seek(&self, seconds: f64, option: SeekOptions) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Seek { seconds, option }).await
    }

    async fn set_loop_file(&self, option: Switch) -> Result<(), MpvError> {
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

    async fn set_loop_playlist(&self, option: Switch) -> Result<(), MpvError> {
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

    async fn set_mute(&self, option: Switch) -> Result<(), MpvError> {
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

    async fn set_speed(
        &self,
        input_speed: f64,
        option: NumberChangeOptions,
    ) -> Result<(), MpvError> {
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

    async fn set_volume(
        &self,
        input_volume: f64,
        option: NumberChangeOptions,
    ) -> Result<(), MpvError> {
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

    async fn stop(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Stop).await
    }
}
