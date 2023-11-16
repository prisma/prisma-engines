use quaint::error::Error as QuaintError;
use wasm_bindgen::JsValue;

type WasmError = JsValue;

/// transforms a Wasm error into a Quaint error
pub(crate) fn into_quaint_error(wasm_err: WasmError) -> QuaintError {
    let status = "WASM_ERROR".to_string();
    let reason = wasm_err.as_string().unwrap_or_else(|| "unknown error".to_string());
    QuaintError::raw_connector_error(status, reason)
}
