use serde::Serialize;
use serde_json::value::Value as JsonValue;

#[derive(Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum JSArg {
    Value(serde_json::Value),
    Buffer(Vec<u8>),
    Array(Vec<JSArg>),
}

impl From<JsonValue> for JSArg {
    fn from(v: JsonValue) -> Self {
        JSArg::Value(v)
    }
}
