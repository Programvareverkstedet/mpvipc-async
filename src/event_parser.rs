//! JSON parsing logic for events from [`MpvIpc`](crate::ipc::MpvIpc).

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{MpvDataType, MpvError, ipc::MpvIpcEvent, message_parser::json_to_value};

/// Reason behind the `MPV_EVENT_END_FILE` event.
///
/// Ref: <https://mpv.io/manual/stable/#command-interface-mpv-event-end-file>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventEndFileReason {
    /// The file has ended. This can (but doesn't have to) include
    /// incomplete files or broken network connections under circumstances.
    Eof,

    /// Playback was ended by a command.
    Stop,

    /// Playback was ended by sending the quit command.
    Quit,

    /// An error happened. In this case, an `error` field is present with the error string.
    Error,

    /// Happens with playlists and similar. For details, see
    /// [`MPV_END_FILE_REASON_REDIRECT`](https://github.com/mpv-player/mpv/blob/72efbfd009a2b3259055133d74b88c81b1115ae1/include/mpv/client.h#L1493)
    /// in the C API.
    Redirect,

    /// Unknown. Normally doesn't happen, unless the Lua API is out of sync
    /// with the C API. (Likewise, it could happen that your script gets reason
    /// strings that did not exist yet at the time your script was written.)
    Unknown,

    /// A catch-all enum variant in case `mpvipc-async` has not implemented the
    /// returned error yet.
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

/// The log level of a log message event.
///
/// Ref:
/// - <https://mpv.io/manual/stable/#command-interface-mpv-event-log-message>
/// - <https://mpv.io/manual/stable/#mp-msg-functions>
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

    /// A catch-all enum variant in case `mpvipc-async` has not implemented the
    /// returned log-level yet.
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
        id: Option<u64>,
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
        match $event.get($key) {
            Some(Value::Null) => None,
            Some(tmp) => Some(
                tmp.$as_type()
                    .ok_or(MpvError::ValueContainsUnexpectedType {
                        expected_type: stringify!($as_type).strip_prefix("as_").unwrap().to_owned(),
                        received: tmp.clone(),
                    })?,
            ),
            None => None,
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

                // TODO: fix these. They are asynchronous responses to different requests.
                //       see:
                //         - https://github.com/mpv-player/mpv/blob/5f768a688b706cf94041adf5bed7c7004af2ec5a/libmpv/client.h#L1158-L1160
                //         - https://github.com/mpv-player/mpv/blob/5f768a688b706cf94041adf5bed7c7004af2ec5a/libmpv/client.h#L1095-L1098
                //         - https://github.com/mpv-player/mpv/blob/5f768a688b706cf94041adf5bed7c7004af2ec5a/libmpv/client.h#L972-L982
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
    let id = get_optional_key_as!(as_u64, "id", event);
    let property_name = get_key_as!(as_str, "name", event);
    let data = event.get("data").map(json_to_value).transpose()?;

    Ok(Event::PropertyChange {
        id,
        name: property_name.to_string(),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::MpvIpcEvent;
    use serde_json::json;

    #[test]
    fn test_parse_simple_events() {
        let simple_events = vec![
            (json!({"event": "file-loaded"}), Event::FileLoaded),
            (json!({"event": "seek"}), Event::Seek),
            (json!({"event": "playback-restart"}), Event::PlaybackRestart),
            (json!({"event": "shutdown"}), Event::Shutdown),
            (json!({"event": "video-reconfig"}), Event::VideoReconfig),
            (json!({"event": "audio-reconfig"}), Event::AudioReconfig),
            (json!({"event": "tick"}), Event::Tick),
            (json!({"event": "idle"}), Event::Idle),
            (json!({"event": "tracks-changed"}), Event::TracksChanged),
            (json!({"event": "track-switched"}), Event::TrackSwitched),
            (json!({"event": "pause"}), Event::Pause),
            (json!({"event": "unpause"}), Event::Unpause),
            (json!({"event": "metadata-update"}), Event::MetadataUpdate),
            (json!({"event": "chapter-change"}), Event::ChapterChange),
        ];

        for (raw_event_json, expected_event) in simple_events {
            let raw_event = MpvIpcEvent(raw_event_json);
            let event = parse_event(raw_event).unwrap();
            assert_eq!(event, expected_event);
        }
    }

    #[test]
    fn test_parse_start_file_event() {
        let raw_event = MpvIpcEvent(json!({
            "event": "start-file",
            "playlist_entry_id": 1
        }));

        let event = parse_event(raw_event).unwrap();

        assert_eq!(
            event,
            Event::StartFile {
                playlist_entry_id: 1
            }
        );
    }

    #[test]
    fn test_parse_end_file_event() {
        let raw_event = MpvIpcEvent(json!({
            "event": "end-file",
            "reason": "eof",
            "playlist_entry_id": 2,
            "file_error": null,
            "playlist_insert_id": 3,
            "playlist_insert_num_entries": 5
        }));
        let event = parse_event(raw_event).unwrap();
        assert_eq!(
            event,
            Event::EndFile {
                reason: EventEndFileReason::Eof,
                playlist_entry_id: 2,
                file_error: None,
                playlist_insert_id: Some(3),
                playlist_insert_num_entries: Some(5)
            }
        );

        let raw_event_with_error = MpvIpcEvent(json!({
            "event": "end-file",
            "reason": "error",
            "playlist_entry_id": 4,
            "file_error": "File not found",
        }));
        let event_with_error = parse_event(raw_event_with_error).unwrap();
        assert_eq!(
            event_with_error,
            Event::EndFile {
                reason: EventEndFileReason::Error,
                playlist_entry_id: 4,
                file_error: Some("File not found".to_string()),
                playlist_insert_id: None,
                playlist_insert_num_entries: None,
            }
        );

        let raw_event_unimplemented = MpvIpcEvent(json!({
            "event": "end-file",
            "reason": "unknown-reason",
            "playlist_entry_id": 5
        }));
        let event_unimplemented = parse_event(raw_event_unimplemented).unwrap();
        assert_eq!(
            event_unimplemented,
            Event::EndFile {
                reason: EventEndFileReason::Unimplemented("unknown-reason".to_string()),
                playlist_entry_id: 5,
                file_error: None,
                playlist_insert_id: None,
                playlist_insert_num_entries: None,
            }
        );
    }

    #[test]
    fn test_parse_log_message_event() {
        let raw_event = MpvIpcEvent(json!({
            "event": "log-message",
            "prefix": "mpv",
            "level": "info",
            "text": "This is a log message"
        }));
        let event = parse_event(raw_event).unwrap();
        assert_eq!(
            event,
            Event::LogMessage {
                prefix: "mpv".to_string(),
                level: EventLogMessageLevel::Info,
                text: "This is a log message".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_hook_event() {
        let raw_event = MpvIpcEvent(json!({
            "event": "hook",
            "hook_id": 42
        }));
        let event = parse_event(raw_event).unwrap();
        assert_eq!(event, Event::Hook { hook_id: 42 });
    }

    #[test]
    fn test_parse_client_message_event() {
        let raw_event = MpvIpcEvent(json!({
            "event": "client-message",
            "args": ["arg1", "arg2", "arg3"]
        }));
        let event = parse_event(raw_event).unwrap();
        assert_eq!(
            event,
            Event::ClientMessage {
                args: vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()]
            }
        );
    }

    #[test]
    fn test_parse_property_change_event() {
        let raw_event = MpvIpcEvent(json!({
            "event": "property-change",
            "id": 1,
            "name": "pause",
            "data": true
        }));
        let event = parse_event(raw_event).unwrap();
        assert_eq!(
            event,
            Event::PropertyChange {
                id: Some(1),
                name: "pause".to_string(),
                data: Some(MpvDataType::Bool(true)),
            }
        );
    }

    #[test]
    fn test_parse_unimplemented_event() {
        let raw_event = MpvIpcEvent(json!({
            "event": "some-unimplemented-event",
            "some_key": "some_value"
        }));
        let event = parse_event(raw_event).unwrap();
        assert_eq!(
            event,
            Event::Unimplemented(
                json!({
                    "event": "some-unimplemented-event",
                    "some_key": "some_value"
                })
                .as_object()
                .unwrap()
                .to_owned()
            )
        );
    }
}
