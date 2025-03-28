#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]

#[cfg(feature = "sqlite-native")]
mod native;
#[cfg(feature = "sqlite-native")]
pub use native::*;

#[cfg(not(feature = "sqlite-native"))]
mod wasm;
#[cfg(not(feature = "sqlite-native"))]
pub use wasm::*;

use super::{acquire_lock, describe_schema, ready};
