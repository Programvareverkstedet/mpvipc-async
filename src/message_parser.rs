use std::collections::HashMap;

use serde_json::Value;

use crate::{Error, ErrorCode, MpvDataType, PlaylistEntry};

pub trait TypeHandler: Sized {
    fn get_value(value: Value) -> Result<Self, Error>;
    fn as_string(&self) -> String;
}

impl TypeHandler for String {
    fn get_value(value: Value) -> Result<String, Error> {
        value
            .as_str()
            .ok_or(Error(ErrorCode::ValueDoesNotContainString))
            .map(|s| s.to_string())
    }

    fn as_string(&self) -> String {
        self.to_string()
    }
}

impl TypeHandler for bool {
    fn get_value(value: Value) -> Result<bool, Error> {
        value
            .as_bool()
            .ok_or(Error(ErrorCode::ValueDoesNotContainBool))
    }

    fn as_string(&self) -> String {
        if *self {
            "true".to_string()
        } else {
            "false".to_string()
        }
    }
}

impl TypeHandler for f64 {
    fn get_value(value: Value) -> Result<f64, Error> {
        value
            .as_f64()
            .ok_or(Error(ErrorCode::ValueDoesNotContainF64))
    }

    fn as_string(&self) -> String {
        self.to_string()
    }
}

impl TypeHandler for usize {
    fn get_value(value: Value) -> Result<usize, Error> {
        value
            .as_u64()
            .map(|u| u as usize)
            .ok_or(Error(ErrorCode::ValueDoesNotContainUsize))
    }

    fn as_string(&self) -> String {
        self.to_string()
    }
}

impl TypeHandler for HashMap<String, MpvDataType> {
    fn get_value(value: Value) -> Result<HashMap<String, MpvDataType>, Error> {
        value
            .as_object()
            .ok_or(Error(ErrorCode::ValueDoesNotContainHashMap))
            .map(json_map_to_hashmap)
    }

    fn as_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl TypeHandler for Vec<PlaylistEntry> {
    fn get_value(value: Value) -> Result<Vec<PlaylistEntry>, Error> {
        value
            .as_array()
            .ok_or(Error(ErrorCode::ValueDoesNotContainPlaylist))
            .map(|array| json_array_to_playlist(array))
    }

    fn as_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub(crate) fn json_map_to_hashmap(
    map: &serde_json::map::Map<String, Value>,
) -> HashMap<String, MpvDataType> {
    let mut output_map: HashMap<String, MpvDataType> = HashMap::new();
    for (ref key, value) in map.iter() {
        match *value {
            Value::Array(ref array) => {
                output_map.insert(
                    key.to_string(),
                    MpvDataType::Array(json_array_to_vec(array)),
                );
            }
            Value::Bool(ref b) => {
                output_map.insert(key.to_string(), MpvDataType::Bool(*b));
            }
            Value::Number(ref n) => {
                if n.is_u64() {
                    output_map.insert(
                        key.to_string(),
                        MpvDataType::Usize(n.as_u64().unwrap() as usize),
                    );
                } else if n.is_f64() {
                    output_map.insert(key.to_string(), MpvDataType::Double(n.as_f64().unwrap()));
                } else {
                    panic!("unimplemented number");
                }
            }
            Value::String(ref s) => {
                output_map.insert(key.to_string(), MpvDataType::String(s.to_string()));
            }
            Value::Object(ref m) => {
                output_map.insert(
                    key.to_string(),
                    MpvDataType::HashMap(json_map_to_hashmap(m)),
                );
            }
            Value::Null => {
                unimplemented!();
            }
        }
    }
    output_map
}

pub(crate) fn json_array_to_vec(array: &[Value]) -> Vec<MpvDataType> {
    let mut output: Vec<MpvDataType> = Vec::new();
    if !array.is_empty() {
        match array[0] {
            Value::Array(_) => {
                for entry in array {
                    if let Value::Array(ref a) = *entry {
                        output.push(MpvDataType::Array(json_array_to_vec(a)));
                    }
                }
            }

            Value::Bool(_) => {
                for entry in array {
                    if let Value::Bool(ref b) = *entry {
                        output.push(MpvDataType::Bool(*b));
                    }
                }
            }

            Value::Number(_) => {
                for entry in array {
                    if let Value::Number(ref n) = *entry {
                        if n.is_u64() {
                            output.push(MpvDataType::Usize(n.as_u64().unwrap() as usize));
                        } else if n.is_f64() {
                            output.push(MpvDataType::Double(n.as_f64().unwrap()));
                        } else {
                            panic!("unimplemented number");
                        }
                    }
                }
            }

            Value::Object(_) => {
                for entry in array {
                    if let Value::Object(ref map) = *entry {
                        output.push(MpvDataType::HashMap(json_map_to_hashmap(map)));
                    }
                }
            }

            Value::String(_) => {
                for entry in array {
                    if let Value::String(ref s) = *entry {
                        output.push(MpvDataType::String(s.to_string()));
                    }
                }
            }

            Value::Null => {
                unimplemented!();
            }
        }
    }
    output
}

pub(crate) fn json_array_to_playlist(array: &[Value]) -> Vec<PlaylistEntry> {
    let mut output: Vec<PlaylistEntry> = Vec::new();
    for (id, entry) in array.iter().enumerate() {
        let mut filename: String = String::new();
        let mut title: String = String::new();
        let mut current: bool = false;
        if let Value::String(ref f) = entry["filename"] {
            filename = f.to_string();
        }
        if let Value::String(ref t) = entry["title"] {
            title = t.to_string();
        }
        if let Value::Bool(ref b) = entry["current"] {
            current = *b;
        }
        output.push(PlaylistEntry {
            id,
            filename,
            title,
            current,
        });
    }
    output
}
