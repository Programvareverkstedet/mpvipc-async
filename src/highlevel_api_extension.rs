//! High-level API extension for [`Mpv`].

use crate::{
    MpvError, IntoRawCommandPart, Mpv, MpvCommand, MpvDataType, Playlist, PlaylistAddOptions,
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
    async fn toggle(&self) -> Result<(), MpvError>;
    async fn stop(&self) -> Result<(), MpvError>;
    async fn set_volume(&self, input_volume: f64, option: NumberChangeOptions)
        -> Result<(), MpvError>;
    async fn set_speed(&self, input_speed: f64, option: NumberChangeOptions) -> Result<(), MpvError>;
    async fn set_mute(&self, option: Switch) -> Result<(), MpvError>;
    async fn set_loop_playlist(&self, option: Switch) -> Result<(), MpvError>;
    async fn set_loop_file(&self, option: Switch) -> Result<(), MpvError>;
    async fn seek(&self, seconds: f64, option: SeekOptions) -> Result<(), MpvError>;
    async fn playlist_shuffle(&self) -> Result<(), MpvError>;
    async fn playlist_remove_id(&self, id: usize) -> Result<(), MpvError>;
    async fn playlist_play_next(&self, id: usize) -> Result<(), MpvError>;
    async fn playlist_play_id(&self, id: usize) -> Result<(), MpvError>;
    async fn playlist_move_id(&self, from: usize, to: usize) -> Result<(), MpvError>;
    async fn playlist_clear(&self) -> Result<(), MpvError>;
    async fn playlist_add(
        &self,
        file: &str,
        file_type: PlaylistAddTypeOptions,
        option: PlaylistAddOptions,
    ) -> Result<(), MpvError>;
    async fn restart(&self) -> Result<(), MpvError>;
    async fn prev(&self) -> Result<(), MpvError>;
    async fn pause(&self) -> Result<(), MpvError>;
    async fn unobserve_property(&self, id: isize) -> Result<(), MpvError>;
    async fn observe_property(&self, id: isize, property: &str) -> Result<(), MpvError>;
    async fn next(&self) -> Result<(), MpvError>;
    async fn kill(&self) -> Result<(), MpvError>;
    async fn get_playlist(&self) -> Result<Playlist, MpvError>;
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

    async fn observe_property(&self, id: isize, property: &str) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Observe {
            id,
            property: property.to_string(),
        })
        .await
    }

    async fn unobserve_property(&self, id: isize) -> Result<(), MpvError> {
        self.run_command(MpvCommand::Unobserve(id)).await
    }

    async fn pause(&self) -> Result<(), MpvError> {
        self.set_property("pause", true).await
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

    async fn set_speed(&self, input_speed: f64, option: NumberChangeOptions) -> Result<(), MpvError> {
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

    async fn toggle(&self) -> Result<(), MpvError> {
        self.run_command_raw("cycle", &["pause"]).await.map(|_| ())
    }
}
