pub mod engine;
pub mod error;
pub mod functions;
pub mod logger;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;

use wasm_bindgen::prelude::wasm_bindgen;

/// Function that should be called before any other public function in this module.
#[wasm_bindgen]
pub fn init() {
    // Set up temporary logging for the wasm module.
    wasm_logger::init(wasm_logger::Config::default());

    // Set up temporary panic hook for the wasm module.
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}
