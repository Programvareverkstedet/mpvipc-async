//! JSON parsing logic for command responses from [`MpvIpc`](crate::ipc::MpvIpc).

use std::collections::HashMap;

use serde_json::Value;

use crate::{MpvDataType, MpvError, PlaylistEntry};

pub trait TypeHandler: Sized {
    fn get_value(value: Value) -> Result<Self, MpvError>;
    fn as_string(&self) -> String;
}

impl TypeHandler for String {
    fn get_value(value: Value) -> Result<String, MpvError> {
        value
            .as_str()
            .ok_or(MpvError::ValueContainsUnexpectedType {
                expected_type: "String".to_string(),
                received: value.clone(),
            })
            .map(|s| s.to_string())
    }

    fn as_string(&self) -> String {
        self.to_string()
    }
}

impl TypeHandler for bool {
    fn get_value(value: Value) -> Result<bool, MpvError> {
        value
            .as_bool()
            .ok_or(MpvError::ValueContainsUnexpectedType {
                expected_type: "bool".to_string(),
                received: value.clone(),
            })
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
    fn get_value(value: Value) -> Result<f64, MpvError> {
        value.as_f64().ok_or(MpvError::ValueContainsUnexpectedType {
            expected_type: "f64".to_string(),
            received: value.clone(),
        })
    }

    fn as_string(&self) -> String {
        self.to_string()
    }
}

impl TypeHandler for usize {
    fn get_value(value: Value) -> Result<usize, MpvError> {
        value
            .as_u64()
            .map(|u| u as usize)
            .ok_or(MpvError::ValueContainsUnexpectedType {
                expected_type: "usize".to_string(),
                received: value.clone(),
            })
    }

    fn as_string(&self) -> String {
        self.to_string()
    }
}

impl TypeHandler for MpvDataType {
    fn get_value(value: Value) -> Result<MpvDataType, MpvError> {
        json_to_value(&value)
    }

    fn as_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl TypeHandler for HashMap<String, MpvDataType> {
    fn get_value(value: Value) -> Result<HashMap<String, MpvDataType>, MpvError> {
        value
            .as_object()
            .ok_or(MpvError::ValueContainsUnexpectedType {
                expected_type: "Map<String, Value>".to_string(),
                received: value.clone(),
            })
            .and_then(json_map_to_hashmap)
    }

    fn as_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl TypeHandler for Vec<PlaylistEntry> {
    fn get_value(value: Value) -> Result<Vec<PlaylistEntry>, MpvError> {
        value
            .as_array()
            .ok_or(MpvError::ValueContainsUnexpectedType {
                expected_type: "Array<Value>".to_string(),
                received: value.clone(),
            })
            .and_then(|array| json_array_to_playlist(array))
    }

    fn as_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub(crate) fn json_to_value(value: &Value) -> Result<MpvDataType, MpvError> {
    match value {
        Value::Array(array) => Ok(MpvDataType::Array(json_array_to_vec(array)?)),
        Value::Bool(b) => Ok(MpvDataType::Bool(*b)),
        Value::Number(n) => {
            if n.is_i64() && n.as_i64().unwrap() == -1 {
                Ok(MpvDataType::MinusOne)
            } else if n.is_u64() {
                Ok(MpvDataType::Usize(n.as_u64().unwrap() as usize))
            } else if n.is_f64() {
                Ok(MpvDataType::Double(n.as_f64().unwrap()))
            } else {
                Err(MpvError::ValueContainsUnexpectedType {
                    expected_type: "i64, u64, or f64".to_string(),
                    received: value.clone(),
                })
            }
        }
        Value::Object(map) => Ok(MpvDataType::HashMap(json_map_to_hashmap(map)?)),
        Value::String(s) => Ok(MpvDataType::String(s.to_string())),
        Value::Null => Ok(MpvDataType::Null),
    }
}

pub(crate) fn json_map_to_hashmap(
    map: &serde_json::map::Map<String, Value>,
) -> Result<HashMap<String, MpvDataType>, MpvError> {
    let mut output_map: HashMap<String, MpvDataType> = HashMap::new();
    for (ref key, value) in map.iter() {
        output_map.insert(key.to_string(), json_to_value(value)?);
    }
    Ok(output_map)
}

pub(crate) fn json_array_to_vec(array: &[Value]) -> Result<Vec<MpvDataType>, MpvError> {
    array.iter().map(json_to_value).collect()
}

fn json_map_to_playlist_entry(
    map: &serde_json::map::Map<String, Value>,
) -> Result<PlaylistEntry, MpvError> {
    let filename = match map.get("filename") {
        Some(Value::String(s)) => s.to_string(),
        Some(data) => {
            return Err(MpvError::ValueContainsUnexpectedType {
                expected_type: "String".to_owned(),
                received: data.clone(),
            })
        }
        None => return Err(MpvError::MissingMpvData),
    };
    let title = match map.get("title") {
        Some(Value::String(s)) => s.to_string(),
        Some(data) => {
            return Err(MpvError::ValueContainsUnexpectedType {
                expected_type: "String".to_owned(),
                received: data.clone(),
            })
        }
        None => return Err(MpvError::MissingMpvData),
    };
    let current = match map.get("current") {
        Some(Value::Bool(b)) => *b,
        Some(data) => {
            return Err(MpvError::ValueContainsUnexpectedType {
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

pub(crate) fn json_array_to_playlist(array: &[Value]) -> Result<Vec<PlaylistEntry>, MpvError> {
    array
        .iter()
        .map(|entry| match entry {
            Value::Object(map) => json_map_to_playlist_entry(map),
            data => Err(MpvError::ValueContainsUnexpectedType {
                expected_type: "Map<String, Value>".to_owned(),
                received: data.clone(),
            }),
        })
        .enumerate()
        .map(|(id, entry)| {
            entry.map(|mut entry| {
                entry.id = id;
                entry
            })
        })
        .collect()
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
            "minus_one": -1,
            "null": null,
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
        expected.insert("minus_one".to_string(), MpvDataType::MinusOne);
        expected.insert("null".to_string(), MpvDataType::Null);
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

        match json_map_to_hashmap(json.as_object().unwrap()) {
            Ok(m) => assert_eq!(m, expected),
            Err(e) => panic!("{:?}", e),
        }
    }

    #[test]
    fn test_json_array_to_vec() {
        let json = json!([
            [1, 2, 3],
            true,
            1.0,
            1,
            -1,
            null,
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
            MpvDataType::MinusOne,
            MpvDataType::Null,
            MpvDataType::String("string".to_string()),
            MpvDataType::HashMap(HashMap::from([(
                "key".to_string(),
                MpvDataType::String("value".to_string()),
            )])),
        ];

        match json_array_to_vec(json.as_array().unwrap()) {
            Ok(v) => assert_eq!(v, expected),
            Err(e) => panic!("{:?}", e),
        }
    }

    #[test]
    fn test_json_array_to_playlist() -> Result<(), MpvError> {
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

        assert_eq!(json_array_to_playlist(json.as_array().unwrap())?, expected);

        Ok(())
    }
}
