//! The JSON value tygr, nom and winnow build, in the shape of nom's and
//! winnow's own `JsonValue`; serde_json's `Value` is converted to it only
//! for checking.

use std::collections::HashMap;

use tygr::{Grammar, StringEq};

use crate::tygr_json::{JsonArray, JsonBool, JsonNumber, JsonObject, JsonString};

/// Also tygr's grammar for a value: each variant converts from its syntax
/// in `tygr_json`.
#[derive(Debug, Clone, PartialEq, Grammar)]
pub(crate) enum JsonValue {
    Object(#[grammar(from = JsonObject)] HashMap<String, JsonValue>),
    Array(#[grammar(from = JsonArray)] Vec<JsonValue>),
    Str(#[grammar(from = JsonString)] String),
    Num(#[grammar(from_str = JsonNumber)] f64),
    Bool(#[grammar(from = JsonBool)] bool),
    #[grammar(from = StringEq!("null"))]
    Null,
}

/// For checking serde_json's own value only, outside the timing.
impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        use serde_json::Value as V;
        match value {
            V::Null => JsonValue::Null,
            V::Bool(b) => JsonValue::Bool(b),
            V::Number(n) => JsonValue::Num(n.as_f64().unwrap()),
            V::String(s) => JsonValue::Str(s),
            V::Array(items) => JsonValue::Array(items.into_iter().map(Into::into).collect()),
            V::Object(members) => {
                JsonValue::Object(members.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}
