//! JSON parsing logic for properties returned in [`Event::PropertyChange`]
//!
//! This module is used to parse the json data from the `data` field of the
//! [`Event::PropertyChange`] variant. Mpv has about 1000 different properties
//! as of `v0.38.0`, so this module will only implement the most common ones.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Event, MpvDataType, MpvError, PlaylistEntry};

/// All possible properties that can be observed through the event system.
///
/// Not all properties are guaranteed to be implemented.
/// If something is missing, please open an issue.
///
/// Otherwise, the property will be returned as a `Property::Unknown` variant.
///
/// See <https://mpv.io/manual/master/#properties> for
/// the upstream list of properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Property {
    Path(Option<String>),
    Pause(bool),
    PlaybackTime(Option<f64>),
    Duration(Option<f64>),
    Metadata(Option<HashMap<String, MpvDataType>>),
    Playlist(Vec<PlaylistEntry>),
    PlaylistPos(Option<usize>),
    LoopFile(LoopProperty),
    LoopPlaylist(LoopProperty),
    Speed(f64),
    Volume(f64),
    Mute(bool),
    Unknown {
        name: String,
        data: Option<MpvDataType>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoopProperty {
    N(usize),
    Inf,
    No,
}

/// Parse a highlevel [`Property`] object from json, used for [`Event::PropertyChange`].
pub fn parse_event_property(event: Event) -> Result<(usize, Property), MpvError> {
    let (id, name, data) = match event {
        Event::PropertyChange { id, name, data } => (id, name, data),
        // TODO: return proper error
        _ => {
            panic!("Event is not a PropertyChange event")
        }
    };

    match name.as_str() {
        "path" => {
            let path = match data {
                Some(MpvDataType::String(s)) => Some(s),
                Some(MpvDataType::Null) => None,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "String".to_owned(),
                        received: data,
                    })
                }
                None => {
                    return Err(MpvError::MissingMpvData);
                }
            };
            Ok((id, Property::Path(path)))
        }
        "pause" => {
            let pause = match data {
                Some(MpvDataType::Bool(b)) => b,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "bool".to_owned(),
                        received: data,
                    })
                }
                None => {
                    return Err(MpvError::MissingMpvData);
                }
            };
            Ok((id, Property::Pause(pause)))
        }
        "playback-time" => {
            let playback_time = match data {
                Some(MpvDataType::Double(d)) => Some(d),
                None | Some(MpvDataType::Null) => None,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::PlaybackTime(playback_time)))
        }
        "duration" => {
            let duration = match data {
                Some(MpvDataType::Double(d)) => Some(d),
                None | Some(MpvDataType::Null) => None,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Duration(duration)))
        }
        "metadata" => {
            let metadata = match data {
                Some(MpvDataType::HashMap(m)) => Some(m),
                None | Some(MpvDataType::Null) => None,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "HashMap".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Metadata(metadata)))
        }
        "playlist" => {
            let playlist = match data {
                Some(MpvDataType::Array(a)) => mpv_array_to_playlist(&a)?,
                None => Vec::new(),
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "Array".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Playlist(playlist)))
        }
        "playlist-pos" => {
            let playlist_pos = match data {
                Some(MpvDataType::Usize(u)) => Some(u),
                Some(MpvDataType::MinusOne) => None,
                Some(MpvDataType::Null) => None,
                None => None,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "usize or -1".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::PlaylistPos(playlist_pos)))
        }
        "loop-file" => {
            let loop_file = match data.to_owned() {
                Some(MpvDataType::Usize(n)) => Some(LoopProperty::N(n)),
                Some(MpvDataType::Bool(b)) => match b {
                    true => Some(LoopProperty::Inf),
                    false => Some(LoopProperty::No),
                },
                Some(MpvDataType::String(s)) => match s.as_str() {
                    "inf" => Some(LoopProperty::Inf),
                    _ => None,
                },
                _ => None,
            }
            .ok_or(match data {
                Some(data) => MpvError::DataContainsUnexpectedType {
                    expected_type: "'inf', bool, or usize".to_owned(),
                    received: data,
                },
                None => MpvError::MissingMpvData,
            })?;
            Ok((id, Property::LoopFile(loop_file)))
        }
        "loop-playlist" => {
            let loop_playlist = match data.to_owned() {
                Some(MpvDataType::Usize(n)) => Some(LoopProperty::N(n)),
                Some(MpvDataType::Bool(b)) => match b {
                    true => Some(LoopProperty::Inf),
                    false => Some(LoopProperty::No),
                },
                Some(MpvDataType::String(s)) => match s.as_str() {
                    "inf" => Some(LoopProperty::Inf),
                    _ => None,
                },
                _ => None,
            }
            .ok_or(match data {
                Some(data) => MpvError::DataContainsUnexpectedType {
                    expected_type: "'inf', bool, or usize".to_owned(),
                    received: data,
                },
                None => MpvError::MissingMpvData,
            })?;

            Ok((id, Property::LoopPlaylist(loop_playlist)))
        }
        "speed" => {
            let speed = match data {
                Some(MpvDataType::Double(d)) => d,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
                None => {
                    return Err(MpvError::MissingMpvData);
                }
            };
            Ok((id, Property::Speed(speed)))
        }
        "volume" => {
            let volume = match data {
                Some(MpvDataType::Double(d)) => d,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
                None => {
                    return Err(MpvError::MissingMpvData);
                }
            };
            Ok((id, Property::Volume(volume)))
        }
        "mute" => {
            let mute = match data {
                Some(MpvDataType::Bool(b)) => b,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "bool".to_owned(),
                        received: data,
                    })
                }
                None => {
                    return Err(MpvError::MissingMpvData);
                }
            };
            Ok((id, Property::Mute(mute)))
        }
        // TODO: add missing cases
        _ => Ok((id, Property::Unknown { name, data })),
    }
}

fn mpv_data_to_playlist_entry(
    map: &HashMap<String, MpvDataType>,
) -> Result<PlaylistEntry, MpvError> {
    let filename = match map.get("filename") {
        Some(MpvDataType::String(s)) => s.to_string(),
        Some(data) => {
            return Err(MpvError::DataContainsUnexpectedType {
                expected_type: "String".to_owned(),
                received: data.clone(),
            })
        }
        None => return Err(MpvError::MissingMpvData),
    };
    let title = match map.get("title") {
        Some(MpvDataType::String(s)) => s.to_string(),
        Some(data) => {
            return Err(MpvError::DataContainsUnexpectedType {
                expected_type: "String".to_owned(),
                received: data.clone(),
            })
        }
        None => return Err(MpvError::MissingMpvData),
    };
    let current = match map.get("current") {
        Some(MpvDataType::Bool(b)) => *b,
        Some(data) => {
            return Err(MpvError::DataContainsUnexpectedType {
                expected_type: "bool".to_owned(),
                received: data.clone(),
            })
        }
        None => return Err(MpvError::MissingMpvData),
    };
    Ok(PlaylistEntry {
        id: 0,
        filename,
        title,
        current,
    })
}

fn mpv_array_to_playlist(array: &[MpvDataType]) -> Result<Vec<PlaylistEntry>, MpvError> {
    array
        .iter()
        .map(|value| match value {
            MpvDataType::HashMap(map) => mpv_data_to_playlist_entry(map),
            _ => Err(MpvError::DataContainsUnexpectedType {
                expected_type: "HashMap".to_owned(),
                received: value.clone(),
            }),
        })
        .enumerate()
        .map(|(id, entry)| entry.map(|entry| PlaylistEntry { id, ..entry }))
        .collect()
}
