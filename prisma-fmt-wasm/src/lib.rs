use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn format(schema: String, params: String) -> String {
    prisma_fmt::format(&schema, &params)
}

/// Docs: https://prisma.github.io/prisma-engines/doc/prisma_fmt/fn.get_config.html
#[wasm_bindgen]
pub fn get_config(params: String) -> String {
    prisma_fmt::get_config(params)
}

#[wasm_bindgen]
pub fn lint(input: String) -> String {
    prisma_fmt::lint(input)
}

#[wasm_bindgen]
pub fn native_types(input: String) -> String {
    prisma_fmt::native_types(input)
}

#[wasm_bindgen]
pub fn referential_actions(input: String) -> String {
    prisma_fmt::referential_actions(input)
}

#[wasm_bindgen]
pub fn preview_features() -> String {
    prisma_fmt::preview_features()
}

/// The API is modelled on an LSP [completion
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_completion).
/// Input and output are both JSON, the request being a `CompletionParams` object and the response
/// being a `CompletionList` object.
#[wasm_bindgen]
pub fn text_document_completion(schema: String, params: String) -> String {
    prisma_fmt::text_document_completion(schema, &params)
}

/// This API is modelled on an LSP [code action
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_codeAction=).
/// Input and output are both JSON, the request being a
/// `CodeActionParams` object and the response being a list of
/// `CodeActionOrCommand` objects.
#[wasm_bindgen]
pub fn code_actions(schema: String, params: String) -> String {
    prisma_fmt::code_actions(schema, &params)
}

#[wasm_bindgen]
pub fn version() -> String {
    String::from("wasm")
}

/// Trigger a panic inside the wasm module. This is only useful in development for testing panic
/// handling.
#[wasm_bindgen]
pub fn debug_panic() {
    panic!("This is the panic triggered by `prisma_fmt::debug_panic()`");
}

#[cfg(feature = "wasm_logger")]
#[wasm_bindgen]
pub fn enable_logs() {
    wasm_logger::init(wasm_logger::Config::default());
}
