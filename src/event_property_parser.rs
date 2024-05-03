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
    Unknown { name: String, data: MpvDataType },
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
                MpvDataType::String(s) => Some(s),
                MpvDataType::Null => None,
                _ => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "String".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Path(path)))
        }
        "pause" => {
            let pause = match data {
                MpvDataType::Bool(b) => b,
                _ => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "bool".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Pause(pause)))
        }
        "playback-time" => {
            let playback_time = match data {
                MpvDataType::Double(d) => Some(d),
                MpvDataType::Null => None,
                _ => {
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
                MpvDataType::Double(d) => Some(d),
                MpvDataType::Null => None,
                _ => {
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
                MpvDataType::HashMap(m) => Some(m),
                MpvDataType::Null => None,
                _ => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "HashMap".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Metadata(metadata)))
        }
        // "playlist" => {
        //     let playlist = match data {
        //         MpvDataType::Array(a) => json_array_to_playlist(&a),
        //         MpvDataType::Null => Vec::new(),
        //         _ => return Err(Error(ErrorCode::ValueDoesNotContainPlaylist)),
        //     };
        //     Ok((id, Property::Playlist(playlist)))
        // }
        "playlist-pos" => {
            let playlist_pos = match data {
                MpvDataType::Usize(u) => Some(u),
                MpvDataType::MinusOne => None,
                MpvDataType::Null => None,
                _ => {
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
                MpvDataType::Usize(n) => Some(LoopProperty::N(n)),
                MpvDataType::Bool(b) => match b {
                    true => Some(LoopProperty::Inf),
                    false => Some(LoopProperty::No),
                },
                MpvDataType::String(s) => match s.as_str() {
                    "inf" => Some(LoopProperty::Inf),
                    _ => None,
                },
                _ => None,
            }
            .ok_or(MpvError::DataContainsUnexpectedType {
                expected_type: "'inf', bool, or usize".to_owned(),
                received: data,
            })?;
            Ok((id, Property::LoopFile(loop_file)))
        }
        "loop-playlist" => {
            let loop_playlist = match data.to_owned() {
                MpvDataType::Usize(n) => Some(LoopProperty::N(n)),
                MpvDataType::Bool(b) => match b {
                    true => Some(LoopProperty::Inf),
                    false => Some(LoopProperty::No),
                },
                MpvDataType::String(s) => match s.as_str() {
                    "inf" => Some(LoopProperty::Inf),
                    _ => None,
                },
                _ => None,
            }
            .ok_or(MpvError::DataContainsUnexpectedType {
                expected_type: "'inf', bool, or usize".to_owned(),
                received: data,
            })?;

            Ok((id, Property::LoopPlaylist(loop_playlist)))
        }
        "speed" => {
            let speed = match data {
                MpvDataType::Double(d) => d,
                _ => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Speed(speed)))
        }
        "volume" => {
            let volume = match data {
                MpvDataType::Double(d) => d,
                _ => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "f64".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Volume(volume)))
        }
        "mute" => {
            let mute = match data {
                MpvDataType::Bool(b) => b,
                _ => {
                    return Err(MpvError::DataContainsUnexpectedType {
                        expected_type: "bool".to_owned(),
                        received: data,
                    })
                }
            };
            Ok((id, Property::Mute(mute)))
        }
        // TODO: add missing cases
        _ => Ok((id, Property::Unknown { name, data })),
    }
}
