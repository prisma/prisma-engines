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

    use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

    // SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
    #[global_allocator]
    static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
        unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

    pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
}

pub use arch::*;
