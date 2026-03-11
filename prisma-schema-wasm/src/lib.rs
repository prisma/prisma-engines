use std::panic;
use wasm_bindgen::prelude::*;

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

/// Handle-based DMMF buffer that holds serialized DMMF JSON as bytes.
///
/// This bypasses V8's string length limit (~536MB / 0x1fffffe8 chars) by keeping the
/// serialized JSON as bytes in WASM linear memory. The JS side reads chunks as `Uint8Array`
/// (which has no V8 string limit) and can reassemble them with a streaming JSON parser.
///
/// Usage from JS:
/// ```js
/// const buffer = get_dmmf_buffered(params);
/// const totalLen = buffer.len();
/// const chunks = [];
/// for (let offset = 0; offset < totalLen; offset += CHUNK_SIZE) {
///     chunks.push(buffer.read_chunk(offset, CHUNK_SIZE));
/// }
/// buffer.free(); // release WASM memory
/// ```
///
/// See: https://github.com/prisma/prisma/issues/29111
#[wasm_bindgen]
pub struct DmmfBuffer {
    data: Vec<u8>,
}

#[wasm_bindgen]
impl DmmfBuffer {
    /// Returns the total byte length of the serialized DMMF JSON.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Read a chunk of the buffer as `Uint8Array`.
    /// `offset` is the byte offset, `length` is the max number of bytes to read.
    /// Returns a `Vec<u8>` which wasm-bindgen converts to `Uint8Array` on the JS side.
    pub fn read_chunk(&self, offset: usize, length: usize) -> Result<Vec<u8>, JsError> {
        if offset >= self.data.len() {
            return Err(JsError::new("Offset beyond buffer length"));
        }
        if length == 0 {
            return Ok(Vec::new());
        }
        // Use saturating_add to avoid overflow on wasm32 (usize is 32-bit)
        let end = std::cmp::min(offset.saturating_add(length), self.data.len());
        Ok(self.data[offset..end].to_vec())
    }
}

/// Serialize DMMF to a caller-owned buffer and return it as a handle.
/// Use `DmmfBuffer.read_chunk()` to read portions as `Uint8Array`, then
/// `DmmfBuffer.free()` (auto-provided by wasm-bindgen) to release WASM memory.
///
/// This avoids implicit global state — each call returns an independent buffer.
///
/// See: https://github.com/prisma/prisma/issues/29111
#[wasm_bindgen]
pub fn get_dmmf_buffered(params: String) -> Result<DmmfBuffer, JsError> {
    register_panic_hook();
    let data = prisma_fmt::get_dmmf_bytes(params).map_err(|e| JsError::new(&e))?;
    Ok(DmmfBuffer { data })
}

/// Trigger a panic inside the wasm module. This is only useful in development for testing panic
/// handling.
#[wasm_bindgen]
pub fn debug_panic() {
    register_panic_hook();
    panic!("This is the panic triggered by `prisma_fmt::debug_panic()`");
}
