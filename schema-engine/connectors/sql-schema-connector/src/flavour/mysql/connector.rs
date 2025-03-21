#[cfg(feature = "mysql-native")]
mod native;
#[cfg(feature = "mysql-native")]
pub use native::*;

#[cfg(not(feature = "mysql-native"))]
mod wasm;
#[cfg(not(feature = "mysql-native"))]
pub use wasm::*;

use super::{Circumstances, Params};
