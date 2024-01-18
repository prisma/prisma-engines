use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::{JsError, JsValue};

// `serialize_missing_as_null` is required to make sure that "empty" values (e.g., `None` and `()`)
//  are serialized as `null` and not `undefined`.
// This is due to certain drivers (e.g., LibSQL) not supporting `undefined` values.
static DEFAULT_SERIALIZER: Serializer = Serializer::new().serialize_missing_as_null(true);

pub(crate) trait ToJsValue: Sized {
    fn to_js_value(&self) -> Result<JsValue, JsValue>;
}

impl<T> ToJsValue for T
where
    T: Serialize,
{
    fn to_js_value(&self) -> Result<JsValue, JsValue> {
        serde_serialize(self)
    }
}

pub(crate) fn serde_serialize<T: Serialize>(value: T) -> Result<JsValue, JsValue> {
    value
        .serialize(&DEFAULT_SERIALIZER)
        .map_err(|err| JsValue::from(JsError::from(err)))
}
