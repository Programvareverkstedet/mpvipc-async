//! JSON parsing logic for command responses from [`MpvIpc`](crate::ipc::MpvIpc).

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
    array
        .iter()
        .map(|entry| match entry {
            Value::Array(a) => MpvDataType::Array(json_array_to_vec(a)),
            Value::Bool(b) => MpvDataType::Bool(*b),
            Value::Number(n) => {
                if n.is_u64() {
                    MpvDataType::Usize(n.as_u64().unwrap() as usize)
                } else if n.is_f64() {
                    MpvDataType::Double(n.as_f64().unwrap())
                } else {
                    panic!("unimplemented number");
                }
            }
            Value::Object(ref o) => MpvDataType::HashMap(json_map_to_hashmap(o)),
            Value::String(s) => MpvDataType::String(s.to_owned()),
            Value::Null => {
                unimplemented!();
            }
        })
        .collect()
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::MpvDataType;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_json_map_to_hashmap() {
        let json = json!({
            "array": [1, 2, 3],
            "bool": true,
            "double": 1.0,
            "usize": 1,
            "string": "string",
            "object": {
                "key": "value"
            }
        });

        let mut expected = HashMap::new();
        expected.insert(
            "array".to_string(),
            MpvDataType::Array(vec![
                MpvDataType::Usize(1),
                MpvDataType::Usize(2),
                MpvDataType::Usize(3),
            ]),
        );
        expected.insert("bool".to_string(), MpvDataType::Bool(true));
        expected.insert("double".to_string(), MpvDataType::Double(1.0));
        expected.insert("usize".to_string(), MpvDataType::Usize(1));
        expected.insert(
            "string".to_string(),
            MpvDataType::String("string".to_string()),
        );
        expected.insert(
            "object".to_string(),
            MpvDataType::HashMap(HashMap::from([(
                "key".to_string(),
                MpvDataType::String("value".to_string()),
            )])),
        );

        assert_eq!(json_map_to_hashmap(json.as_object().unwrap()), expected);
    }

    #[test]
    #[should_panic]
    fn test_json_map_to_hashmap_fail_on_null() {
        json_map_to_hashmap(
            json!({
                "null": null
            })
            .as_object()
            .unwrap(),
        );
    }

    #[test]
    fn test_json_array_to_vec() {
        let json = json!([
            [1, 2, 3],
            true,
            1.0,
            1,
            "string",
            {
                "key": "value"
            }
        ]);

        println!("{:?}", json.as_array().unwrap());
        println!("{:?}", json_array_to_vec(json.as_array().unwrap()));

        let expected = vec![
            MpvDataType::Array(vec![
                MpvDataType::Usize(1),
                MpvDataType::Usize(2),
                MpvDataType::Usize(3),
            ]),
            MpvDataType::Bool(true),
            MpvDataType::Double(1.0),
            MpvDataType::Usize(1),
            MpvDataType::String("string".to_string()),
            MpvDataType::HashMap(HashMap::from([(
                "key".to_string(),
                MpvDataType::String("value".to_string()),
            )])),
        ];

        assert_eq!(json_array_to_vec(json.as_array().unwrap()), expected);
    }

    #[test]
    #[should_panic]
    fn test_json_array_to_vec_fail_on_null() {
        json_array_to_vec(json!([null]).as_array().unwrap().as_slice());
    }

    #[test]
    fn test_json_array_to_playlist() {
        let json = json!([
            {
                "filename": "file1",
                "title": "title1",
                "current": true
            },
            {
                "filename": "file2",
                "title": "title2",
                "current": false
            }
        ]);

        let expected = vec![
            PlaylistEntry {
                id: 0,
                filename: "file1".to_string(),
                title: "title1".to_string(),
                current: true,
            },
            PlaylistEntry {
                id: 1,
                filename: "file2".to_string(),
                title: "title2".to_string(),
                current: false,
            },
        ];

        assert_eq!(json_array_to_playlist(json.as_array().unwrap()), expected);
    }
}
