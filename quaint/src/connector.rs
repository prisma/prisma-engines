//! A set of abstractions for database connections.
//!
//! Provides traits for database querying and executing, and for spawning
//! transactions.
//!
//! Connectors for [MySQL](struct.Mysql.html),
//! [PostgreSQL](struct.PostgreSql.html), [SQLite](struct.Sqlite.html) and [SQL
//! Server](struct.Mssql.html) connect to the corresponding databases and
//! implement the [Queryable](trait.Queryable.html) trait for generalized
//! querying interface.

mod connection_info;

pub mod metrics;
mod queryable;
mod result_set;
#[cfg(any(
    feature = "mssql-connector",
    feature = "postgresql-connector",
    feature = "mysql-connector"
))]
mod timeout;
mod transaction;
mod type_identifier;

#[cfg(feature = "mssql-connector")]
pub(crate) mod mssql;
#[cfg(feature = "mssql")]
pub(crate) mod mssql_wasm;
// #[cfg(feature = "mysql-connector")]
// pub(crate) mod mysql;
// #[cfg(feature = "mysql")]
// pub(crate) mod mysql_wasm;
// #[cfg(feature = "postgresql-connector")]
// pub(crate) mod postgres;
// #[cfg(feature = "postgresql")]
// pub(crate) mod postgres_wasm;
#[cfg(feature = "sqlite-connector")]
pub(crate) mod sqlite;
#[cfg(feature = "sqlite")]
pub(crate) mod sqlite_wasm;

// #[cfg(feature = "mysql-connector")]
// pub use self::mysql::*;
// #[cfg(feature = "mysql")]
// pub use self::mysql_wasm::*;
// #[cfg(feature = "postgresql-connector")]
// pub use self::postgres::*;
// #[cfg(feature = "postgresql")]
// pub use self::postgres_wasm::*;
#[cfg(feature = "mssql-connector")]
pub use mssql::*;
#[cfg(feature = "mssql")]
pub use mssql_wasm::*;
#[cfg(feature = "sqlite-connector")]
pub use sqlite::*;
#[cfg(feature = "sqlite")]
pub use sqlite_wasm::*;

pub use self::result_set::*;
pub use connection_info::*;
pub use queryable::*;
pub use transaction::*;
#[cfg(any(
    feature = "mssql-connector",
    feature = "postgresql-connector",
    feature = "mysql-connector"
))]
#[allow(unused_imports)]
pub(crate) use type_identifier::*;

pub use self::metrics::query;

#[cfg(feature = "postgresql")]
pub(crate) mod postgres;
#[cfg(feature = "postgresql-connector")]
pub use postgres::native::*;
#[cfg(feature = "postgresql")]
pub use postgres::wasm::common::*;

#[cfg(feature = "mysql")]
pub(crate) mod mysql;
#[cfg(feature = "mysql-connector")]
pub use mysql::native::*;
#[cfg(feature = "mysql")]
pub use mysql::wasm::common::*;
