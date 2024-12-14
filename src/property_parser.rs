//! JSON parsing logic for properties returned by
//! [`Event::PropertyChange`], and used internally in `MpvExt`
//! to parse the response from `Mpv::get_property()`.
//!
//! This module is used to parse the json data from the `data` field of
//! known properties. Mpv has about 1000 different properties
//! as of `v0.38.0`, so this module will only implement the most common ones.

// TODO: reuse this logic for providing a more typesafe response API to `Mpv::get_property()`
//       Although this data is currently of type `Option<`

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{MpvDataType, MpvError, PlaylistEntry};

/// An incomplete list of properties that mpv can return.
///
/// Unimplemented properties will be returned with it's data
/// as a `Property::Unknown` variant.
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
    TimePos(Option<f64>),
    TimeRemaining(Option<f64>),
    Speed(f64),
    Volume(f64),
    Mute(bool),
    EofReached(bool),
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

/// Parse a highlevel [`Property`] object from mpv data.
///
/// This is intended to be used with the `data` field of
/// `Event::PropertyChange` and the response from `Mpv::get_property_value()`.
pub fn parse_property(name: &str, data: Option<MpvDataType>) -> Result<Property, MpvError> {
    match name {
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
            Ok(Property::Path(path))
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
            Ok(Property::Pause(pause))
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
            Ok(Property::PlaybackTime(playback_time))
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
            Ok(Property::Duration(duration))
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
            Ok(Property::Metadata(metadata))
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
            Ok(Property::Playlist(playlist))
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
            Ok(Property::PlaylistPos(playlist_pos))
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
            Ok(Property::LoopFile(loop_file))
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

            Ok(Property::LoopPlaylist(loop_playlist))
        }
        "time-pos" => {
            let time_pos = match data {
                Some(MpvDataType::Double(d)) => Some(d),
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
                None => None,
            };

            Ok(Property::TimePos(time_pos))
        }
        "time-remaining" => {
            let time_remaining = match data {
                Some(MpvDataType::Double(d)) => Some(d),
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
                None => None,
            };
            Ok(Property::TimeRemaining(time_remaining))
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
            Ok(Property::Speed(speed))
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
            Ok(Property::Volume(volume))
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
            Ok(Property::Mute(mute))
        }
        "eof-reached" => {
            let eof_reached = match data {
                Some(MpvDataType::Bool(b)) => b,
                Some(data) => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "bool".to_owned(),
                        received: data,
                    })
                }
                None => true,
            };
            Ok(Property::EofReached(eof_reached))
        }
        // TODO: add missing cases
        _ => Ok(Property::Unknown {
            name: name.to_owned(),
            data,
        }),
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
        Some(MpvDataType::String(s)) => Some(s.to_string()),
        Some(data) => {
            return Err(MpvError::DataContainsUnexpectedType {
                expected_type: "String".to_owned(),
                received: data.clone(),
            })
        }
        None => None,
    };
    let current = match map.get("current") {
        Some(MpvDataType::Bool(b)) => *b,
        Some(data) => {
            return Err(MpvError::DataContainsUnexpectedType {
                expected_type: "bool".to_owned(),
                received: data.clone(),
            })
        }
        None => false,
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
