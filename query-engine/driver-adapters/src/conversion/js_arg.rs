use serde_json::value::Value as JsonValue;

#[derive(Debug, PartialEq)]
pub enum JSArg {
    SafeInt(i32),
    Value(serde_json::Value),
    Buffer(Vec<u8>),
    Array(Vec<JSArg>),
}

impl From<JsonValue> for JSArg {
    fn from(v: JsonValue) -> Self {
        JSArg::Value(v)
    }
}
