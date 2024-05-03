//! JSON parsing logic for events from [`MpvIpc`](crate::ipc::MpvIpc).

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{ipc::MpvIpcEvent, message_parser::json_to_value, MpvDataType, MpvError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventEndFileReason {
    Eof,
    Stop,
    Quit,
    Error,
    Redirect,
    Unknown,
    Unimplemented(String),
}

impl FromStr for EventEndFileReason {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eof" => Ok(EventEndFileReason::Eof),
            "stop" => Ok(EventEndFileReason::Stop),
            "quit" => Ok(EventEndFileReason::Quit),
            "error" => Ok(EventEndFileReason::Error),
            "redirect" => Ok(EventEndFileReason::Redirect),
            reason => Ok(EventEndFileReason::Unimplemented(reason.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventLogMessageLevel {
    Info,
    Warn,
    Error,
    Fatal,
    Verbose,
    Debug,
    Trace,
    Unimplemented(String),
}

impl FromStr for EventLogMessageLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(EventLogMessageLevel::Info),
            "warn" => Ok(EventLogMessageLevel::Warn),
            "error" => Ok(EventLogMessageLevel::Error),
            "fatal" => Ok(EventLogMessageLevel::Fatal),
            "verbose" => Ok(EventLogMessageLevel::Verbose),
            "debug" => Ok(EventLogMessageLevel::Debug),
            "trace" => Ok(EventLogMessageLevel::Trace),
            level => Ok(EventLogMessageLevel::Unimplemented(level.to_string())),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    StartFile {
        playlist_entry_id: usize,
    },
    EndFile {
        reason: EventEndFileReason,
        playlist_entry_id: usize,
        file_error: Option<String>,
        playlist_insert_id: Option<usize>,
        playlist_insert_num_entries: Option<usize>,
    },
    FileLoaded,
    Seek,
    PlaybackRestart,
    Shutdown,
    LogMessage {
        prefix: String,
        level: EventLogMessageLevel,
        text: String,
    },
    Hook {
        hook_id: usize,
    },
    GetPropertyReply,
    SetPropertyReply,
    CommandReply {
        result: String,
    },
    ClientMessage {
        args: Vec<String>,
    },
    VideoReconfig,
    AudioReconfig,
    PropertyChange {
        id: usize,
        name: String,
        data: Option<MpvDataType>,
    },
    EventQueueOverflow,
    None,

    /// Deprecated since mpv v0.33.0
    Idle,

    /// Deprecated since mpv v0.31.0
    Tick,

    /// Deprecated since mpv v0.7.0, removed in mpv v0.35.0
    TracksChanged,

    /// Deprecated since mpv v0.7.0, removed in mpv v0.35.0
    TrackSwitched,

    /// Deprecated since mpv v0.7.0, removed in mpv v0.35.0
    Pause,

    /// Deprecated since mpv v0.7.0, removed in mpv v0.35.0
    Unpause,

    /// Deprecated since mpv v0.7.0, removed in mpv v0.35.0
    MetadataUpdate,

    /// Deprecated since mpv v0.7.0, removed in mpv v0.35.0
    ChapterChange,

    /// Deprecated since mpv v0.7.0, removed in mpv v0.35.0
    ScriptInputDispatch,

    /// Catch-all for unimplemented events
    Unimplemented(Map<String, Value>),
}

macro_rules! get_key_as {
    ($as_type:ident, $key:expr, $event:ident) => {{
        let tmp = $event.get($key).ok_or(MpvError::MissingKeyInObject {
            key: $key.to_owned(),
            map: $event.clone(),
        })?;

        tmp.$as_type()
            .ok_or(MpvError::ValueContainsUnexpectedType {
                expected_type: stringify!($as_type).strip_prefix("as_").unwrap().to_owned(),
                received: tmp.clone(),
            })?
    }};
}

macro_rules! get_optional_key_as {
    ($as_type:ident, $key:expr, $event:ident) => {{
        if let Some(tmp) = $event.get($key) {
            Some(
                tmp.$as_type()
                    .ok_or(MpvError::ValueContainsUnexpectedType {
                        expected_type: stringify!($as_type).strip_prefix("as_").unwrap().to_owned(),
                        received: tmp.clone(),
                    })?,
            )
        } else {
            None
        }
    }};
}

// NOTE: I have not been able to test all of these events,
//       so some of the parsing logic might be incorrect.
//       In particular, I have not been able to make mpv
//       produce any of the commented out events, and since
//       the documentation for the most part just says
//       "See C API", I have not pursued this further.
//
//       If you need this, please open an issue or a PR.

/// Parse a highlevel [`Event`] objects from json.
pub(crate) fn parse_event(raw_event: MpvIpcEvent) -> Result<Event, MpvError> {
    let MpvIpcEvent(event) = raw_event;

    event
        .as_object()
        .ok_or(MpvError::ValueContainsUnexpectedType {
            expected_type: "object".to_owned(),
            received: event.clone(),
        })
        .and_then(|event| {
            let event_name = get_key_as!(as_str, "event", event);

            match event_name {
                "start-file" => parse_start_file(event),
                "end-file" => parse_end_file(event),
                "file-loaded" => Ok(Event::FileLoaded),
                "seek" => Ok(Event::Seek),
                "playback-restart" => Ok(Event::PlaybackRestart),
                "shutdown" => Ok(Event::Shutdown),
                "log-message" => parse_log_message(event),
                "hook" => parse_hook(event),
                // "get-property-reply" =>
                // "set-property-reply" =>
                // "command-reply" =>
                "client-message" => parse_client_message(event),
                "video-reconfig" => Ok(Event::VideoReconfig),
                "audio-reconfig" => Ok(Event::AudioReconfig),
                "property-change" => parse_property_change(event),
                "tick" => Ok(Event::Tick),
                "idle" => Ok(Event::Idle),
                "tracks-changed" => Ok(Event::TracksChanged),
                "track-switched" => Ok(Event::TrackSwitched),
                "pause" => Ok(Event::Pause),
                "unpause" => Ok(Event::Unpause),
                "metadata-update" => Ok(Event::MetadataUpdate),
                "chapter-change" => Ok(Event::ChapterChange),
                _ => Ok(Event::Unimplemented(event.to_owned())),
            }
        })
}

fn parse_start_file(event: &Map<String, Value>) -> Result<Event, MpvError> {
    let playlist_entry_id = get_key_as!(as_u64, "playlist_entry_id", event) as usize;

    Ok(Event::StartFile { playlist_entry_id })
}

fn parse_end_file(event: &Map<String, Value>) -> Result<Event, MpvError> {
    let reason = get_key_as!(as_str, "reason", event);
    let playlist_entry_id = get_key_as!(as_u64, "playlist_entry_id", event) as usize;
    let file_error = get_optional_key_as!(as_str, "file_error", event).map(|s| s.to_string());
    let playlist_insert_id =
        get_optional_key_as!(as_u64, "playlist_insert_id", event).map(|i| i as usize);
    let playlist_insert_num_entries =
        get_optional_key_as!(as_u64, "playlist_insert_num_entries", event).map(|i| i as usize);

    Ok(Event::EndFile {
        reason: reason
            .parse()
            .unwrap_or(EventEndFileReason::Unimplemented(reason.to_string())),
        playlist_entry_id,
        file_error,
        playlist_insert_id,
        playlist_insert_num_entries,
    })
}

fn parse_log_message(event: &Map<String, Value>) -> Result<Event, MpvError> {
    let prefix = get_key_as!(as_str, "prefix", event).to_owned();
    let level = get_key_as!(as_str, "level", event);
    let text = get_key_as!(as_str, "text", event).to_owned();

    Ok(Event::LogMessage {
        prefix,
        level: level
            .parse()
            .unwrap_or(EventLogMessageLevel::Unimplemented(level.to_string())),
        text,
    })
}

fn parse_hook(event: &Map<String, Value>) -> Result<Event, MpvError> {
    let hook_id = get_key_as!(as_u64, "hook_id", event) as usize;
    Ok(Event::Hook { hook_id })
}

fn parse_client_message(event: &Map<String, Value>) -> Result<Event, MpvError> {
    let args = get_key_as!(as_array, "args", event)
        .iter()
        .map(|arg| {
            arg.as_str()
                .ok_or(MpvError::ValueContainsUnexpectedType {
                    expected_type: "string".to_owned(),
                    received: arg.clone(),
                })
                .map(|s| s.to_string())
        })
        .collect::<Result<Vec<String>, MpvError>>()?;
    Ok(Event::ClientMessage { args })
}

fn parse_property_change(event: &Map<String, Value>) -> Result<Event, MpvError> {
    let id = get_key_as!(as_u64, "id", event) as usize;
    let property_name = get_key_as!(as_str, "name", event);
    let data = event.get("data").map(|d| json_to_value(d)).transpose()?;

    Ok(Event::PropertyChange {
        id,
        name: property_name.to_string(),
        data: data,
    })
}
