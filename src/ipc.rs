use crate::message_parser::TypeHandler;

use self::message_parser::extract_mpv_response_data;
use self::message_parser::json_array_to_playlist;
use self::message_parser::json_array_to_vec;
use self::message_parser::json_map_to_hashmap;

use super::*;
use log::{debug, warn};
use serde_json::json;
use serde_json::Value;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;

pub fn get_mpv_property<T: TypeHandler>(instance: &Mpv, property: &str) -> Result<T, Error> {
    let ipc_string = json!({"command": ["get_property", property]});
    match serde_json::from_str::<Value>(&send_command_sync(instance, ipc_string)) {
        Ok(val) => T::get_value(val),
        Err(why) => Err(Error(ErrorCode::JsonParseError(why.to_string()))),
    }
}

pub fn get_mpv_property_string(instance: &Mpv, property: &str) -> Result<String, Error> {
    let ipc_string = json!({"command": ["get_property", property]});
    let val = serde_json::from_str::<Value>(&send_command_sync(instance, ipc_string))
        .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())))?;

    let data = extract_mpv_response_data(&val)?;

    match data {
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(ref n) => Ok(n.to_string()),
        Value::String(ref s) => Ok(s.to_string()),
        Value::Array(ref array) => Ok(format!("{:?}", array)),
        Value::Object(ref map) => Ok(format!("{:?}", map)),
        Value::Null => Err(Error(ErrorCode::MissingValue)),
    }
}

fn validate_mpv_response(response: &str) -> Result<(), Error> {
    serde_json::from_str::<Value>(response)
        .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())))
        .and_then(|value| extract_mpv_response_data(&value).map(|_| ()))
}

pub fn set_mpv_property(instance: &Mpv, property: &str, value: Value) -> Result<(), Error> {
    let ipc_string = json!({
        "command": ["set_property", property, value]
    });

    let response = &send_command_sync(instance, ipc_string);
    validate_mpv_response(response)
}

pub fn run_mpv_command(instance: &Mpv, command: &str, args: &[&str]) -> Result<(), Error> {
    let mut ipc_string = json!({
        "command": [command]
    });
    if let Value::Array(args_array) = &mut ipc_string["command"] {
        for arg in args {
            args_array.push(json!(arg));
        }
    }

    let response = &send_command_sync(instance, ipc_string);
    validate_mpv_response(response)
}

pub fn observe_mpv_property(instance: &Mpv, id: &isize, property: &str) -> Result<(), Error> {
    let ipc_string = json!({
        "command": ["observe_property", id, property]
    });

    let response = &send_command_sync(instance, ipc_string);
    validate_mpv_response(response)
}

pub fn unobserve_mpv_property(instance: &Mpv, id: &isize) -> Result<(), Error> {
    let ipc_string = json!({
        "command": ["unobserve_property", id]
    });

    let response = &send_command_sync(instance, ipc_string);
    validate_mpv_response(response)
}

fn try_convert_property(name: &str, id: usize, data: MpvDataType) -> Event {
    let property = match name {
        "path" => match data {
            MpvDataType::String(value) => Property::Path(Some(value)),
            MpvDataType::Null => Property::Path(None),
            _ => unimplemented!(),
        },
        "pause" => match data {
            MpvDataType::Bool(value) => Property::Pause(value),
            _ => unimplemented!(),
        },
        "playback-time" => match data {
            MpvDataType::Double(value) => Property::PlaybackTime(Some(value)),
            MpvDataType::Null => Property::PlaybackTime(None),
            _ => unimplemented!(),
        },
        "duration" => match data {
            MpvDataType::Double(value) => Property::Duration(Some(value)),
            MpvDataType::Null => Property::Duration(None),
            _ => unimplemented!(),
        },
        "metadata" => match data {
            MpvDataType::HashMap(value) => Property::Metadata(Some(value)),
            MpvDataType::Null => Property::Metadata(None),
            _ => unimplemented!(),
        },
        _ => {
            warn!("Property {} not implemented", name);
            Property::Unknown {
                name: name.to_string(),
                data,
            }
        }
    };
    Event::PropertyChange { id, property }
}

pub fn listen(instance: &mut Mpv) -> Result<Event, Error> {
    let mut e;
    // sometimes we get responses unrelated to events, so we read a new line until we receive one
    // with an event field
    let name = loop {
        let mut response = String::new();
        instance.reader.read_line(&mut response).unwrap();
        response = response.trim_end().to_string();
        debug!("Event: {}", response);

        e = serde_json::from_str::<Value>(&response)
            .map_err(|why| Error(ErrorCode::JsonParseError(why.to_string())))?;

        match e["event"] {
            Value::String(ref name) => break name,
            _ => {
                // It was not an event - try again
                debug!("Bad response: {:?}", response)
            }
        }
    };

    let event = match name.as_str() {
        "shutdown" => Event::Shutdown,
        "start-file" => Event::StartFile,
        "file-loaded" => Event::FileLoaded,
        "seek" => Event::Seek,
        "playback-restart" => Event::PlaybackRestart,
        "idle" => Event::Idle,
        "tick" => Event::Tick,
        "video-reconfig" => Event::VideoReconfig,
        "audio-reconfig" => Event::AudioReconfig,
        "tracks-changed" => Event::TracksChanged,
        "track-switched" => Event::TrackSwitched,
        "pause" => Event::Pause,
        "unpause" => Event::Unpause,
        "metadata-update" => Event::MetadataUpdate,
        "chapter-change" => Event::ChapterChange,
        "end-file" => Event::EndFile,
        "property-change" => {
            let name = match e["name"] {
                Value::String(ref n) => Ok(n.to_string()),
                _ => Err(Error(ErrorCode::JsonContainsUnexptectedType)),
            }?;

            let id: usize = match e["id"] {
                Value::Number(ref n) => n.as_u64().unwrap() as usize,
                _ => 0,
            };

            let data: MpvDataType = match e["data"] {
                Value::String(ref n) => MpvDataType::String(n.to_string()),

                Value::Array(ref a) => {
                    if name == "playlist".to_string() {
                        MpvDataType::Playlist(Playlist(json_array_to_playlist(a)))
                    } else {
                        MpvDataType::Array(json_array_to_vec(a))
                    }
                }

                Value::Bool(b) => MpvDataType::Bool(b),

                Value::Number(ref n) => {
                    if n.is_u64() {
                        MpvDataType::Usize(n.as_u64().unwrap() as usize)
                    } else if n.is_f64() {
                        MpvDataType::Double(n.as_f64().unwrap())
                    } else {
                        return Err(Error(ErrorCode::JsonContainsUnexptectedType));
                    }
                }

                Value::Object(ref m) => MpvDataType::HashMap(json_map_to_hashmap(m)),

                Value::Null => MpvDataType::Null,
            };

            try_convert_property(name.as_ref(), id, data)
        }
        "client-message" => {
            let args = match e["args"] {
                Value::Array(ref a) => json_array_to_vec(a)
                    .iter()
                    .map(|arg| match arg {
                        MpvDataType::String(s) => Ok(s.to_owned()),
                        _ => Err(Error(ErrorCode::JsonContainsUnexptectedType)),
                    })
                    .collect::<Result<Vec<_>, _>>(),
                _ => return Err(Error(ErrorCode::JsonContainsUnexptectedType)),
            }?;
            Event::ClientMessage { args }
        }
        _ => Event::Unimplemented,
    };
    Ok(event)
}

pub fn listen_raw(instance: &mut Mpv) -> String {
    let mut response = String::new();
    instance.reader.read_line(&mut response).unwrap();
    response.trim_end().to_string()
}

fn send_command_sync(instance: &Mpv, command: Value) -> String {
    let stream = &instance.stream;
    match serde_json::to_writer(stream, &command) {
        Err(why) => panic!("Error: Could not write to socket: {}", why),
        Ok(_) => {
            let mut stream = stream;
            stream.write_all(b"\n").unwrap();
            let mut response = String::new();
            {
                let mut reader = BufReader::new(stream);
                while !response.contains("\"error\":") {
                    response.clear();
                    reader.read_line(&mut response).unwrap();
                }
            }
            debug!("Response: {}", response.trim_end());
            response
        }
    }
}
