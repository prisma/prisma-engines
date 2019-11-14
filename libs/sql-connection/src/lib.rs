#![deny(rust_2018_idioms)]

//! Shared SQL connection handling logic for the migration engine and the introspection engine.

mod generic_sql_connection;
mod traits;

pub use generic_sql_connection::GenericSqlConnection;
pub use traits::*;

use tokio::runtime::Runtime;

fn default_runtime() -> Runtime {
    Runtime::new().expect("failed to start tokio runtime")
}
