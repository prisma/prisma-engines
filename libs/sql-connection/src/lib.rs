#![deny(rust_2018_idioms)]

//! Shared SQL connection handling logic for the migration engine and the introspection engine.

mod connection_info;
mod generic_sql_connection;
mod mysql;
mod postgres;
mod sqlite;
mod traits;

pub use connection_info::ConnectionInfo;
pub use generic_sql_connection::{GenericSqlConnection, SqlFamily, SUPPORTED_SCHEMES};
pub use mysql::*;
pub use postgres::*;
pub use sqlite::*;
pub use traits::*;

use tokio::runtime::Runtime;

fn default_runtime() -> Runtime {
    Runtime::new().expect("failed to start tokio runtime")
}
