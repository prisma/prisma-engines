pub(crate) type JsonObject = serde_json::Map<String, serde_json::Value>;
pub(crate) type JsonValue = serde_json::Value;

pub(crate) trait JsonValueExt {
    fn into_object(self) -> Option<JsonObject>;
    fn as_object_mut(&mut self) -> Option<&mut JsonObject>;

    fn into_list(self) -> Option<Vec<JsonValue>>;
    fn as_list_mut(&mut self) -> Option<&mut Vec<JsonValue>>;
}

impl JsonValueExt for JsonValue {
    fn into_object(self) -> Option<JsonObject> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn as_object_mut(&mut self) -> Option<&mut JsonObject> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn into_list(self) -> Option<Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn as_list_mut(&mut self) -> Option<&mut Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }
}
