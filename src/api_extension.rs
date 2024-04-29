use crate::{Error, IntoRawCommandPart, Mpv, MpvCommand, MpvDataType, Playlist, PlaylistAddOptions, PlaylistAddTypeOptions, PlaylistEntry, SeekOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub enum Switch {
    On,
    Off,
    Toggle,
}

// TODO: fix this
#[allow(async_fn_in_trait)]
pub trait MpvExt {
    async fn toggle(&self) -> Result<(), Error>;
    async fn stop(&self) -> Result<(), Error>;
    async fn set_volume(&self, input_volume: f64, option: NumberChangeOptions) -> Result<(), Error>;
    async fn set_speed(&self, input_speed: f64, option: NumberChangeOptions) -> Result<(), Error>;
    async fn set_mute(&self, option: Switch) -> Result<(), Error>;
    async fn set_loop_playlist(&self, option: Switch) -> Result<(), Error>;
    async fn set_loop_file(&self, option: Switch) -> Result<(), Error>;
    async fn seek(&self, seconds: f64, option: SeekOptions) -> Result<(), Error>;
    async fn playlist_shuffle(&self) -> Result<(), Error>;
    async fn playlist_remove_id(&self, id: usize) -> Result<(), Error>;
    async fn playlist_play_next(&self, id: usize) -> Result<(), Error>;
    async fn playlist_play_id(&self, id: usize) -> Result<(), Error>;
    async fn playlist_move_id(&self, from: usize, to: usize) -> Result<(), Error>;
    async fn playlist_clear(&self) -> Result<(), Error>;
    async fn playlist_add(&self, file: &str, file_type: PlaylistAddTypeOptions, option: PlaylistAddOptions) -> Result<(), Error>;
    async fn restart(&self) -> Result<(), Error>;
    async fn prev(&self) -> Result<(), Error>;
    async fn pause(&self) -> Result<(), Error>;
    async fn unobserve_property(&self, id: isize) -> Result<(), Error>;
    async fn observe_property(&self, id: isize, property: &str) -> Result<(), Error>;
    async fn next(&self) -> Result<(), Error>;
    async fn kill(&self) -> Result<(), Error>;
    async fn get_playlist(&self) -> Result<Playlist, Error>;
    async fn get_metadata(&self) -> Result<HashMap<String, MpvDataType>, Error>;

}

impl MpvExt for Mpv {
    async fn get_metadata(&self) -> Result<HashMap<String, MpvDataType>, Error> {
        self.get_property("metadata").await
    }

    async fn get_playlist(&self) -> Result<Playlist, Error> {
        self.get_property::<Vec<PlaylistEntry>>("playlist")
            .await
            .map(|entries| Playlist(entries))
    }

    async fn kill(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::Quit).await
    }

    async fn next(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistNext).await
    }

    async fn observe_property(&self, id: isize, property: &str) -> Result<(), Error> {
        self.run_command(MpvCommand::Observe {
            id,
            property: property.to_string(),
        })
        .await
    }

    async fn unobserve_property(&self, id: isize) -> Result<(), Error> {
        self.run_command(MpvCommand::Unobserve(id)).await
    }

    async fn pause(&self) -> Result<(), Error> {
        self.set_property("pause", true).await
    }

    async fn prev(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistPrev).await
    }

    async fn restart(&self) -> Result<(), Error> {
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

    async fn playlist_clear(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistClear).await
    }

    async fn playlist_move_id(&self, from: usize, to: usize) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistMove { from, to })
            .await
    }

    async fn playlist_play_id(&self, id: usize) -> Result<(), Error> {
        self.set_property("playlist-pos", id).await
    }

    async fn playlist_play_next(&self, id: usize) -> Result<(), Error> {
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

    async fn playlist_remove_id(&self, id: usize) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistRemove(id)).await
    }

    async fn playlist_shuffle(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::PlaylistShuffle).await
    }

    async fn seek(&self, seconds: f64, option: SeekOptions) -> Result<(), Error> {
        self.run_command(MpvCommand::Seek { seconds, option }).await
    }

    async fn set_loop_file(&self, option: Switch) -> Result<(), Error> {
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

    async fn set_loop_playlist(&self, option: Switch) -> Result<(), Error> {
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

    async fn set_mute(&self, option: Switch) -> Result<(), Error> {
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

    async fn set_volume(
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

    async fn stop(&self) -> Result<(), Error> {
        self.run_command(MpvCommand::Stop).await
    }

    async fn toggle(&self) -> Result<(), Error> {
        self.run_command_raw("cycle", &["pause"]).await.map(|_| ())
    }
}