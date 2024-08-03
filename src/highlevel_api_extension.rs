//! High-level API extension for [`Mpv`].

use crate::{
    parse_property, IntoRawCommandPart, LoopProperty, Mpv, MpvCommand, MpvDataType, MpvError,
    Playlist, PlaylistAddOptions, Property, SeekOptions,
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
    // COMMANDS

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

    /// Stop the player completely (as opposed to pausing),
    /// removing the pointer to the current video.
    async fn stop(&self) -> Result<(), MpvError>;

    // SETTERS

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

    // GETTERS

    /// Get a list of all entries in the playlist.
    async fn get_playlist(&self) -> Result<Playlist, MpvError>;

    /// Get metadata about the current video.
    async fn get_metadata(&self) -> Result<HashMap<String, MpvDataType>, MpvError>;

    /// Get the path of the current video.
    async fn get_file_path(&self) -> Result<String, MpvError>;

    /// Get the current volume of the player.
    async fn get_volume(&self) -> Result<f64, MpvError>;

    /// Get the playback speed of the player.
    async fn get_speed(&self) -> Result<f64, MpvError>;

    /// Get the current position in the current video.
    async fn get_time_pos(&self) -> Result<Option<f64>, MpvError>;

    /// Get the amount of time remaining in the current video.
    async fn get_time_remaining(&self) -> Result<Option<f64>, MpvError>;

    /// Get the total duration of the current video.
    async fn get_duration(&self) -> Result<f64, MpvError>;

    /// Get the current position in the playlist.
    async fn get_playlist_pos(&self) -> Result<usize, MpvError>;

    // BOOLEAN GETTERS

    /// Check whether the player is muted.
    async fn is_muted(&self) -> Result<bool, MpvError>;

    /// Check whether the player is currently playing.
    async fn is_playing(&self) -> Result<bool, MpvError>;

    /// Check whether the player is looping the current playlist.
    async fn playlist_is_looping(&self) -> Result<LoopProperty, MpvError>;

    /// Check whether the player is looping the current video.
    async fn file_is_looping(&self) -> Result<LoopProperty, MpvError>;
}

impl MpvExt for Mpv {
    // COMMANDS

    async fn seek(&self, seconds: f64, option: SeekOptions) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Seek { seconds, option }).await
    }

    async fn playlist_shuffle(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistShuffle).await
    }

    async fn playlist_remove_id(&self, id: usize) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistRemove(id)).await
    }

    async fn playlist_play_next(&self, id: usize) -> Result<(), MpvError> {
        let data = self.get_property("playlist-pos").await?;
        let current_id = match parse_property("playlist-pos", data)? {
            Property::PlaylistPos(Some(current_id)) => Ok(current_id),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }?;

        self.run_command(MpvCommand::PlaylistMove {
            from: id,
            to: current_id + 1,
        })
        .await
    }

    async fn playlist_play_id(&self, id: usize) -> Result<(), MpvError> {
        self.set_property("playlist-pos", id).await
    }

    async fn playlist_move_id(&self, from: usize, to: usize) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistMove { from, to })
            .await
    }

    async fn playlist_clear(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistClear).await
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

    async fn restart(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Seek {
            seconds: 0f64,
            option: SeekOptions::Absolute,
        })
        .await
    }

    async fn prev(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistPrev).await
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

    async fn next(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::PlaylistNext).await
    }

    async fn kill(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Quit).await
    }

    async fn stop(&self) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Stop).await
    }

    // SETTERS

    async fn set_volume(
        &self,
        input_volume: f64,
        option: NumberChangeOptions,
    ) -> Result<(), MpvError> {
        let volume = self.get_volume().await?;

        match option {
            NumberChangeOptions::Increase => {
                self.set_property("volume", volume + input_volume).await
            }
            NumberChangeOptions::Decrease => {
                self.set_property("volume", volume - input_volume).await
            }
            NumberChangeOptions::Absolute => self.set_property("volume", input_volume).await,
        }
    }

    async fn set_speed(
        &self,
        input_speed: f64,
        option: NumberChangeOptions,
    ) -> Result<(), MpvError> {
        let speed = self.get_speed().await?;

        match option {
            NumberChangeOptions::Increase => self.set_property("speed", speed + input_speed).await,
            NumberChangeOptions::Decrease => self.set_property("speed", speed - input_speed).await,
            NumberChangeOptions::Absolute => self.set_property("speed", input_speed).await,
        }
    }

    async fn set_playback(&self, option: Switch) -> Result<(), MpvError> {
        let enabled = match option {
            Switch::On => "no",
            Switch::Off => "yes",
            Switch::Toggle => {
                if self.is_playing().await? {
                    "no"
                } else {
                    "yes"
                }
            }
        };
        self.set_property("pause", enabled).await
    }

    async fn set_mute(&self, option: Switch) -> Result<(), MpvError> {
        let enabled = match option {
            Switch::On => "yes",
            Switch::Off => "no",
            Switch::Toggle => {
                if self.is_muted().await? {
                    "no"
                } else {
                    "yes"
                }
            }
        };
        self.set_property("mute", enabled).await
    }

    async fn set_loop_playlist(&self, option: Switch) -> Result<(), MpvError> {
        let enabled = match option {
            Switch::On => "inf",
            Switch::Off => "no",
            Switch::Toggle => match self.playlist_is_looping().await? {
                LoopProperty::Inf => "no",
                LoopProperty::N(_) => "no",
                LoopProperty::No => "inf",
            },
        };
        self.set_property("loop-playlist", enabled).await
    }

    async fn set_loop_file(&self, option: Switch) -> Result<(), MpvError> {
        let enabled = match option {
            Switch::On => "inf",
            Switch::Off => "no",
            Switch::Toggle => match self.file_is_looping().await? {
                LoopProperty::Inf => "no",
                LoopProperty::N(_) => "no",
                LoopProperty::No => "inf",
            },
        };
        self.set_property("loop-file", enabled).await
    }

    // GETTERS

    async fn get_playlist(&self) -> Result<Playlist, MpvError> {
        let data = self.get_property("playlist").await?;
        match parse_property("playlist", data)? {
            Property::Playlist(value) => Ok(Playlist(value)),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_metadata(&self) -> Result<HashMap<String, MpvDataType>, MpvError> {
        let data = self.get_property("metadata").await?;
        match parse_property("metadata", data)? {
            Property::Metadata(Some(value)) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_file_path(&self) -> Result<String, MpvError> {
        let data = self.get_property("path").await?;
        match parse_property("path", data)? {
            Property::Path(Some(value)) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_volume(&self) -> Result<f64, MpvError> {
        let data = self.get_property("volume").await?;
        match parse_property("volume", data)? {
            Property::Volume(value) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_speed(&self) -> Result<f64, MpvError> {
        let data = self.get_property("speed").await?;
        match parse_property("speed", data)? {
            Property::Speed(value) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_time_pos(&self) -> Result<Option<f64>, MpvError> {
        let data = self.get_property("time-pos").await?;
        match parse_property("time-pos", data)? {
            Property::TimePos(value) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_time_remaining(&self) -> Result<Option<f64>, MpvError> {
        let data = self.get_property("time-remaining").await?;
        match parse_property("time-remaining", data)? {
            Property::TimeRemaining(value) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_duration(&self) -> Result<f64, MpvError> {
        let data = self.get_property("duration").await?;
        match parse_property("duration", data)? {
            Property::Duration(Some(value)) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn get_playlist_pos(&self) -> Result<usize, MpvError> {
        let data = self.get_property("playlist-pos").await?;
        match parse_property("playlist-pos", data)? {
            Property::PlaylistPos(Some(value)) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    // BOOLEAN GETTERS

    async fn is_muted(&self) -> Result<bool, MpvError> {
        let data = self.get_property("mute").await?;
        match parse_property("mute", data)? {
            Property::Mute(value) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn is_playing(&self) -> Result<bool, MpvError> {
        let data = self.get_property("pause").await?;
        match parse_property("pause", data)? {
            Property::Pause(value) => Ok(!value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn playlist_is_looping(&self) -> Result<LoopProperty, MpvError> {
        let data = self.get_property("loop-playlist").await?;
        match parse_property("loop-playlist", data)? {
            Property::LoopPlaylist(value) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }

    async fn file_is_looping(&self) -> Result<LoopProperty, MpvError> {
        let data = self.get_property("loop-file").await?;
        match parse_property("loop-file", data)? {
            Property::LoopFile(value) => Ok(value),
            prop => Err(MpvError::UnexpectedProperty(prop)),
        }
    }
}
