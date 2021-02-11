#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod sql;

mod assertions;
mod test_api;

pub use assertions::*;
pub use test_api::*;
pub use test_macros::test_each_connector;
pub use test_setup::*;

pub type TestResult = anyhow::Result<()>;
