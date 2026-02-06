use std::panic;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

/// Global buffer for storing DMMF bytes when using the buffered API.
/// This allows JS to read the DMMF in chunks via Uint8Array, bypassing
/// V8's string length limit of ~536MB.
/// See: https://github.com/prisma/prisma/issues/29111
static DMMF_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[wasm_bindgen]
extern "C" {
    /// This function registers the reason for a Wasm panic via the
    /// JS function `globalThis.PRISMA_WASM_PANIC_REGISTRY.set_message()`
    #[wasm_bindgen(js_namespace = ["global", "PRISMA_WASM_PANIC_REGISTRY"], js_name = "set_message")]
    fn prisma_set_wasm_panic_message(s: &str);
}

/// Registers a singleton panic hook that will register the reason for the Wasm panic in JS.
/// Without this, the panic message would be lost: you'd see `RuntimeError: unreachable` message in JS,
/// with no reference to the Rust function and line that panicked.
/// This function should be manually called before any other public function in this module.
/// Note: no method is safe to call after a panic has occurred.
fn register_panic_hook() {
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();

    SET_HOOK.call_once(|| {
        #[cfg(feature = "wasm-logger")]
        wasm_logger::init(wasm_logger::Config::default());

        panic::set_hook(Box::new(|info| {
            let message = &info.to_string();
            prisma_set_wasm_panic_message(message);
        }));
    });
}

#[wasm_bindgen]
pub fn format(schema: String, params: String) -> String {
    register_panic_hook();
    prisma_fmt::format(schema, &params)
}

/// Docs: https://prisma.github.io/prisma-engines/doc/prisma_fmt/fn.get_config.html
#[wasm_bindgen]
pub fn get_config(params: String) -> String {
    register_panic_hook();
    prisma_fmt::get_config(params)
}

/// Docs: https://prisma.github.io/prisma-engines/doc/prisma_fmt/fn.get_dmmf.html
#[wasm_bindgen]
pub fn get_dmmf(params: String) -> Result<String, JsError> {
    register_panic_hook();
    prisma_fmt::get_dmmf(params).map_err(|e| JsError::new(&e))
}

#[wasm_bindgen]
pub fn get_datamodel(params: String) -> Result<String, JsError> {
    register_panic_hook();
    prisma_fmt::get_datamodel(params).map_err(|e| JsError::new(&e))
}

#[wasm_bindgen]
pub fn lint(input: String) -> String {
    register_panic_hook();
    prisma_fmt::lint(input)
}

#[wasm_bindgen]
pub fn validate(params: String) -> Result<(), JsError> {
    register_panic_hook();
    prisma_fmt::validate(params).map_err(|e| JsError::new(&e))
}

#[wasm_bindgen]
pub fn merge_schemas(input: String) -> Result<String, JsError> {
    register_panic_hook();
    prisma_fmt::merge_schemas(input).map_err(|e| JsError::new(&e))
}

#[wasm_bindgen]
pub fn native_types(input: String) -> String {
    register_panic_hook();
    prisma_fmt::native_types(input)
}

#[wasm_bindgen]
pub fn referential_actions(input: String) -> String {
    register_panic_hook();
    prisma_fmt::referential_actions(input)
}

#[wasm_bindgen]
pub fn preview_features() -> String {
    register_panic_hook();
    prisma_fmt::preview_features()
}

/// The API is modelled on an LSP [completion
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#completion-request-leftwards_arrow_with_hook).
/// Input and output are both JSON, the request being a `CompletionParams` object and the response
/// being a `CompletionList` object.
#[wasm_bindgen]
pub fn text_document_completion(schema_files: String, params: String) -> String {
    register_panic_hook();
    prisma_fmt::text_document_completion(schema_files, &params)
}

/// This API is modelled on an LSP [code action
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#code-action-request-leftwards_arrow_with_hook).
/// Input and output are both JSON, the request being a
/// `CodeActionParams` object and the response being a list of
/// `CodeActionOrCommand` objects.
#[wasm_bindgen]
pub fn code_actions(schema: String, params: String) -> String {
    register_panic_hook();
    prisma_fmt::code_actions(schema, &params)
}

/// This API is modelled on an LSP [references
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#find-references-request-leftwards_arrow_with_hook).
/// Input and output are both JSON, the request being a
/// `CodeActionParams` object and the response being a list of
/// `CodeActionOrCommand` objects.
#[wasm_bindgen]
pub fn references(schema: String, params: String) -> String {
    register_panic_hook();
    prisma_fmt::references(schema, &params)
}

/// This api is modelled on an LSP [hover request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#hover-request-leftwards_arrow_with_hook).
/// Input and output are both JSON, the request being a `HoverParams` object
/// and the response being a `Hover` object.
#[wasm_bindgen]
pub fn hover(schema_files: String, params: String) -> String {
    register_panic_hook();
    prisma_fmt::hover(schema_files, &params)
}

/// Serialize DMMF to an internal buffer and return the total byte count.
/// Use `read_dmmf_chunk()` to read portions as Uint8Array, then `free_dmmf_buffer()` to release.
///
/// This bypasses V8's string length limit (~536MB / 0x1fffffe8 chars) by keeping the
/// serialized JSON as bytes in WASM linear memory. The JS side reads chunks as Uint8Array
/// (which has no V8 string limit) and can use a streaming JSON parser.
///
/// See: https://github.com/prisma/prisma/issues/29111
#[wasm_bindgen]
pub fn get_dmmf_buffered(params: String) -> Result<usize, JsError> {
    register_panic_hook();
    let bytes = prisma_fmt::get_dmmf_bytes(params).map_err(|e| JsError::new(&e))?;
    let len = bytes.len();
    let mut buf = DMMF_BUFFER.lock().unwrap();
    *buf = bytes;
    Ok(len)
}

/// Read a chunk of the DMMF buffer as Uint8Array.
/// `offset` is the byte offset, `length` is the number of bytes to read.
/// Returns a Vec<u8> which wasm-bindgen converts to Uint8Array on the JS side.
#[wasm_bindgen]
pub fn read_dmmf_chunk(offset: usize, length: usize) -> Result<Vec<u8>, JsError> {
    register_panic_hook();
    let buf = DMMF_BUFFER.lock().unwrap();
    if offset >= buf.len() {
        return Err(JsError::new("Offset beyond buffer length"));
    }
    let end = std::cmp::min(offset + length, buf.len());
    Ok(buf[offset..end].to_vec())
}

/// Free the internal DMMF buffer. Call this after reading all chunks.
#[wasm_bindgen]
pub fn free_dmmf_buffer() {
    register_panic_hook();
    let mut buf = DMMF_BUFFER.lock().unwrap();
    *buf = Vec::new();
}

/// Trigger a panic inside the wasm module. This is only useful in development for testing panic
/// handling.
#[wasm_bindgen]
pub fn debug_panic() {
    register_panic_hook();
    panic!("This is the panic triggered by `prisma_fmt::debug_panic()`");
}
