#[cfg(not(target_arch = "wasm32"))]
mod arch {
    // This crate only works in a Wasm environment.
    // This conditional compilation block is here to make commands like
    // `cargo clippy --all-features` happy, as `clippy` doesn't support the
    // `--exclude` option (see: https://github.com/rust-lang/rust-clippy/issues/9555).
    //
    // This crate can still be inspected by `clippy` via:
    // `cargo clippy --all-features -p query-engine-wasm --target wasm32-unknown-unknown`
}

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
mod arch {
    pub use super::wasm::*;

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
}

pub use arch::*;
