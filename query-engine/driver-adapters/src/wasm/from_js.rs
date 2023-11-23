use serde::de::DeserializeOwned;
use wasm_bindgen::JsValue;

pub trait FromJsValue: Sized {
    fn from_js_value(value: JsValue) -> Result<Self, JsValue>;
}

impl<T> FromJsValue for T
where
    T: DeserializeOwned,
{
    fn from_js_value(value: JsValue) -> Result<Self, JsValue> {
        serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from(e))
    }
}
