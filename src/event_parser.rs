use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{ipc::MpvIpcEvent, Error, ErrorCode, MpvDataType};

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

pub(crate) fn map_event(raw_event: MpvIpcEvent) -> Result<Event, Error> {
    let MpvIpcEvent(event) = raw_event;

    event
        .as_object()
        .ok_or(Error(ErrorCode::JsonContainsUnexptectedType))
        .and_then(|event| {
            let event_name = event
                .get("event")
                .ok_or(Error(ErrorCode::MissingValue))?
                .as_str()
                .ok_or(Error(ErrorCode::ValueDoesNotContainString))?;

            match event_name {
                "shutdown" => Ok(Event::Shutdown),
                "start-file" => Ok(Event::StartFile),
                "end-file" => Ok(Event::EndFile),
                "file-loaded" => Ok(Event::FileLoaded),
                "tracks-changed" => Ok(Event::TracksChanged),
                "track-switched" => Ok(Event::TrackSwitched),
                "idle" => Ok(Event::Idle),
                "pause" => Ok(Event::Pause),
                "unpause" => Ok(Event::Unpause),
                "tick" => Ok(Event::Tick),
                "video-reconfig" => Ok(Event::VideoReconfig),
                "audio-reconfig" => Ok(Event::AudioReconfig),
                "metadata-update" => Ok(Event::MetadataUpdate),
                "seek" => Ok(Event::Seek),
                "playback-restart" => Ok(Event::PlaybackRestart),
                "property-change" => {
                    let id = event
                        .get("id")
                        .ok_or(Error(ErrorCode::MissingValue))?
                        .as_u64()
                        .ok_or(Error(ErrorCode::ValueDoesNotContainUsize))?
                        as usize;
                    let property_name = event
                        .get("name")
                        .ok_or(Error(ErrorCode::MissingValue))?
                        .as_str()
                        .ok_or(Error(ErrorCode::ValueDoesNotContainString))?;

                    match property_name {
                        "path" => {
                            let path = event
                                .get("data")
                                .ok_or(Error(ErrorCode::MissingValue))?
                                .as_str()
                                .map(|s| s.to_string());
                            Ok(Event::PropertyChange {
                                id,
                                property: Property::Path(path),
                            })
                        }
                        "pause" => {
                            let pause = event
                                .get("data")
                                .ok_or(Error(ErrorCode::MissingValue))?
                                .as_bool()
                                .ok_or(Error(ErrorCode::ValueDoesNotContainBool))?;
                            Ok(Event::PropertyChange {
                                id,
                                property: Property::Pause(pause),
                            })
                        }
                        // TODO: missing cases
                        _ => {
                            let data = event
                                .get("data")
                                .ok_or(Error(ErrorCode::MissingValue))?
                                .clone();
                            Ok(Event::PropertyChange {
                                id,
                                property: Property::Unknown {
                                    name: property_name.to_string(),
                                    // TODO: fix
                                    data: MpvDataType::Double(data.as_f64().unwrap_or(0.0)),
                                },
                            })
                        }
                    }
                }
                "chapter-change" => Ok(Event::ChapterChange),
                "client-message" => {
                    let args = event
                        .get("args")
                        .ok_or(Error(ErrorCode::MissingValue))?
                        .as_array()
                        .ok_or(Error(ErrorCode::ValueDoesNotContainString))?
                        .iter()
                        .map(|arg| {
                            arg.as_str()
                                .ok_or(Error(ErrorCode::ValueDoesNotContainString))
                                .map(|s| s.to_string())
                        })
                        .collect::<Result<Vec<String>, Error>>()?;
                    Ok(Event::ClientMessage { args })
                }
                _ => Ok(Event::Unimplemented),
            }
        })
}
