use serde::Serialize;
use wasm_bindgen::{JsError, JsValue};

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
        .serialize(&shared_wasm::RESPONSE_WITH_BIGINT_SERIALIZER)
        .map_err(|err| JsValue::from(JsError::from(err)))
}
