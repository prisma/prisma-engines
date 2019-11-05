#![deny(missing_docs, rust_2018_idioms)]

//! Shared SQL connection handling logic for the migration engine and the introspection engine.

mod mysql;
mod pooling;
mod postgres;
mod sqlite;
mod traits;

pub use mysql::*;
pub use postgres::*;
pub use sqlite::*;
pub use traits::*;

use tokio::runtime::Runtime;

fn default_runtime() -> Runtime {
    Runtime::new().expect("failed to start tokio runtime")
}
