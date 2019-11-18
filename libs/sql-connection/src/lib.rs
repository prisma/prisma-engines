#![deny(rust_2018_idioms)]

//! Shared SQL connection handling logic for the migration engine and the introspection engine.

mod generic_sql_connection;

pub use generic_sql_connection::GenericSqlConnection;
pub use quaint::prelude::Queryable;
