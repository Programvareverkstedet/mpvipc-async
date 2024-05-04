//! Library specific error messages.

use serde_json::{Map, Value};
use thiserror::Error;

use crate::{MpvDataType, Property};

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

    #[error(
        "Mpv sent data with an unexpected type:\nExpected {expected_type}, received {received:#?}"
    )]
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

    #[error("Unexpected property: {0:?}")]
    UnexpectedProperty(Property),

    #[error("Unknown error: {0}")]
    Other(String),
}

impl PartialEq for MpvError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MpvError(l0), Self::MpvError(r0)) => l0 == r0,
            (Self::MpvSocketConnectionError(l0), Self::MpvSocketConnectionError(r0)) => l0 == r0,
            (Self::InternalConnectionError(l0), Self::InternalConnectionError(r0)) => l0 == r0,
            (Self::JsonParseError(l0), Self::JsonParseError(r0)) => {
                l0.to_string() == r0.to_string()
            }
            (
                Self::ValueContainsUnexpectedType {
                    expected_type: l_expected_type,
                    received: l_received,
                },
                Self::ValueContainsUnexpectedType {
                    expected_type: r_expected_type,
                    received: r_received,
                },
            ) => l_expected_type == r_expected_type && l_received == r_received,
            (
                Self::DataContainsUnexpectedType {
                    expected_type: l_expected_type,
                    received: l_received,
                },
                Self::DataContainsUnexpectedType {
                    expected_type: r_expected_type,
                    received: r_received,
                },
            ) => l_expected_type == r_expected_type && l_received == r_received,
            (
                Self::MissingKeyInObject {
                    key: l_key,
                    map: l_map,
                },
                Self::MissingKeyInObject {
                    key: r_key,
                    map: r_map,
                },
            ) => l_key == r_key && l_map == r_map,

            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
