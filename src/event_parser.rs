//! JSON parsing logic for events from [`MpvIpc`](crate::ipc::MpvIpc).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{ipc::MpvIpcEvent, Error, ErrorCode, MpvDataType};

/// All possible properties that can be observed through the event system.
///
/// Not all properties are guaranteed to be implemented.
/// If something is missing, please open an issue.
///
/// Otherwise, the property will be returned as a `Property::Unknown` variant.
///
/// See <https://mpv.io/manual/master/#properties> for
/// the upstream list of properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Property {
    Path(Option<String>),
    Pause(bool),
    PlaybackTime(Option<f64>),
    Duration(Option<f64>),
    Metadata(Option<HashMap<String, MpvDataType>>),
    Unknown { name: String, data: MpvDataType },
}

/// All possible events that can be sent by mpv.
///
/// Not all event types are guaranteed to be implemented.
/// If something is missing, please open an issue.
///
/// Otherwise, the event will be returned as an `Event::Unimplemented` variant.
///
/// See <https://mpv.io/manual/master/#list-of-events> for
/// the upstream list of events.
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

/// Parse a highlevel [`Event`] objects from json.
pub(crate) fn parse_event(raw_event: MpvIpcEvent) -> Result<Event, Error> {
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
                "property-change" => parse_event_property(event)
                    .map(|(id, property)| Event::PropertyChange { id, property }),
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

/// Parse a highlevel [`Property`] object from json, used for [`Event::PropertyChange`].
fn parse_event_property(event: &Map<String, Value>) -> Result<(usize, Property), Error> {
    let id = event
        .get("id")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_u64()
        .ok_or(Error(ErrorCode::ValueDoesNotContainUsize))? as usize;
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
            Ok((id, Property::Path(path)))
        }
        "pause" => {
            let pause = event
                .get("data")
                .ok_or(Error(ErrorCode::MissingValue))?
                .as_bool()
                .ok_or(Error(ErrorCode::ValueDoesNotContainBool))?;
            Ok((id, Property::Pause(pause)))
        }
        // TODO: missing cases
        _ => {
            let data = event
                .get("data")
                .ok_or(Error(ErrorCode::MissingValue))?
                .clone();
            Ok((
                id,
                Property::Unknown {
                    name: property_name.to_string(),
                    // TODO: fix
                    data: MpvDataType::Double(data.as_f64().unwrap_or(0.0)),
                },
            ))
        }
    }
}
