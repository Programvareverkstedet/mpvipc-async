//! JSON parsing logic for events from [`MpvIpc`](crate::ipc::MpvIpc).

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{ipc::MpvIpcEvent, message_parser::json_to_value, Error, ErrorCode, MpvDataType};

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
        data: MpvDataType,
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

// NOTE: I have not been able to test all of these events,
//       so some of the parsing logic might be incorrect.
//       In particular, I have not been able to make mpv
//       produce any of the commented out events, and since
//       the documentation for the most part just says
//       "See C API", I have not pursued this further.
//
//       If you need this, please open an issue or a PR.

/// Parse a highlevel [`Event`] objects from json.
#[allow(deprecated)]
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

fn parse_start_file(event: &Map<String, Value>) -> Result<Event, Error> {
    let playlist_entry_id = event
        .get("playlist_entry_id")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_u64()
        .ok_or(Error(ErrorCode::ValueDoesNotContainUsize))? as usize;
    Ok(Event::StartFile { playlist_entry_id })
}

fn parse_end_file(event: &Map<String, Value>) -> Result<Event, Error> {
    let reason = event
        .get("reason")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_str()
        .ok_or(Error(ErrorCode::ValueDoesNotContainString))?;
    let playlist_entry_id = event
        .get("playlist_entry_id")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_u64()
        .ok_or(Error(ErrorCode::ValueDoesNotContainUsize))? as usize;
    let file_error = event
        .get("file_error")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let playlist_insert_id = event
        .get("playlist_insert_id")
        .and_then(|v| v.as_u64().map(|u| u as usize));
    let playlist_insert_num_entries = event
        .get("playlist_insert_num_entries")
        .and_then(|v| v.as_u64().map(|u| u as usize));

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

fn parse_log_message(event: &Map<String, Value>) -> Result<Event, Error> {
    let prefix = event
        .get("prefix")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_str()
        .ok_or(Error(ErrorCode::ValueDoesNotContainString))?
        .to_string();
    let level = event
        .get("level")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_str()
        .ok_or(Error(ErrorCode::ValueDoesNotContainString))?;
    let text = event
        .get("text")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_str()
        .ok_or(Error(ErrorCode::ValueDoesNotContainString))?
        .to_string();

    Ok(Event::LogMessage {
        prefix,
        level: level
            .parse()
            .unwrap_or(EventLogMessageLevel::Unimplemented(level.to_string())),
        text,
    })
}

fn parse_hook(event: &Map<String, Value>) -> Result<Event, Error> {
    let hook_id = event
        .get("hook_id")
        .ok_or(Error(ErrorCode::MissingValue))?
        .as_u64()
        .ok_or(Error(ErrorCode::ValueDoesNotContainUsize))? as usize;
    Ok(Event::Hook { hook_id })
}

fn parse_client_message(event: &Map<String, Value>) -> Result<Event, Error> {
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

fn parse_property_change(event: &Map<String, Value>) -> Result<Event, Error> {
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
    let data = event
        .get("data")
        .ok_or(Error(ErrorCode::MissingValue))?
        .clone();

    Ok(Event::PropertyChange {
        id,
        name: property_name.to_string(),
        data: json_to_value(&data)?,
    })
}
