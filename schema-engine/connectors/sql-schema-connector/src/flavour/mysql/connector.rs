#[cfg(feature = "mysql-native")]
mod native;
#[cfg(feature = "mysql-native")]
pub use native::*;

#[cfg(not(feature = "mysql-native"))]
pub mod wasm;
#[cfg(not(feature = "mysql-native"))]
use wasm::*;

use super::{Circumstances, Params};
