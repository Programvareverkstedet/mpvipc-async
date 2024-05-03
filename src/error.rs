//! Library specific error messages.

use thiserror::Error;
use serde_json::{Map, Value};

use crate::MpvDataType;

/// Any error that can occur when interacting with mpv.
#[derive(Error, Debug)]
pub enum MpvError {
    #[error("MpvError: {0}")]
    MpvError(String),

    #[error("Error communicating over mpv socket: {0}")]
    MpvSocketConnectionError(String),

    #[error("Internal connection error: {0}")]
    InternalConnectionError(String),

    #[error("JsonParseError: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Mpv sent a value with an unexpected type:\nExpected {expected_type}, received {received:#?}")]
    ValueContainsUnexpectedType {
      expected_type: String,
      received: Value,
    },

    #[error("Mpv sent data with an unexpected type:\nExpected {expected_type}, received {received:#?}")]
    DataContainsUnexpectedType {
      expected_type: String,
      received: MpvDataType,
    },

    #[error("Missing expected 'data' field in mpv message")]
    MissingMpvData,

    #[error("Missing key in object:\nExpected {key} in {map:#?}")]
    MissingKeyInObject {
      key: String,
      map: Map<String, Value>,
    },

    #[error("Unknown error: {0}")]
    Other(String),
}