#[cfg(feature = "mssql-native")]
mod native;
#[cfg(feature = "mssql-native")]
pub use native::*;

#[cfg(not(feature = "mssql-native"))]
mod wasm;
#[cfg(not(feature = "mssql-native"))]
pub use wasm::*;

use super::Params;
