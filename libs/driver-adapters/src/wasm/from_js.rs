use serde::de::DeserializeOwned;
use wasm_bindgen::JsValue;

pub(crate) trait FromJsValue: Sized {
    fn from_js_value(value: JsValue) -> Result<Self, JsValue>;
}

impl<T> FromJsValue for T
where
    T: DeserializeOwned,
{
    fn from_js_value(value: JsValue) -> Result<Self, JsValue> {
        let value = if value.is_undefined() { JsValue::null() } else { value };
        serde_wasm_bindgen::from_value(value).map_err(JsValue::from)
    }
}
