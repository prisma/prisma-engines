mod common;
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use crate::wasm::*;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::native::*;

pub use crate::common::regex::RegExpCompat;
pub use crate::common::spawn::SpawnError;
