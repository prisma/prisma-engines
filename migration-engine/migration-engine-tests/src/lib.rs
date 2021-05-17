#![deny(rust_2018_idioms, unsafe_code)]

pub mod multi_engine_test_api;
pub mod sql;
pub mod sync_test_api;

mod assertions;
mod test_api;

pub use assertions::*;
pub use test_api::*;
pub use test_macros::test_connector;
pub use test_setup::*;

pub type TestResult = anyhow::Result<()>;
