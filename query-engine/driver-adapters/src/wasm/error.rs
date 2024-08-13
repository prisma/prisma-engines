use js_sys::Reflect;
use quaint::error::Error as QuaintError;
use wasm_bindgen::JsValue;

/// transforms a Wasm error into a Quaint error
pub(crate) fn into_quaint_error(wasm_err: JsValue) -> QuaintError {
    let status = "WASM_ERROR".to_string();
    let reason = Reflect::get(&wasm_err, &JsValue::from_str("stack"))
        .ok()
        .and_then(|value| value.as_string())
        .unwrap_or_else(|| "Unknown error".to_string());
    QuaintError::raw_connector_error(status, reason)
}
