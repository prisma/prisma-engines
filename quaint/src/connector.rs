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
#[cfg(any(feature = "mssql", feature = "postgresql", feature = "mysql"))]
mod timeout;
mod transaction;
mod type_identifier;

#[cfg(feature = "mssql")]
pub(crate) mod mssql;
#[cfg(feature = "mysql")]
pub(crate) mod mysql;
#[cfg(feature = "postgresql")]
pub(crate) mod postgres;
#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;

#[cfg(feature = "mysql")]
pub use self::mysql::*;
#[cfg(feature = "postgresql")]
pub use self::postgres::*;
pub use self::result_set::*;
pub use connection_info::*;
#[cfg(feature = "mssql")]
pub use mssql::*;
pub use queryable::*;
#[cfg(feature = "sqlite")]
pub use sqlite::*;
pub use transaction::*;
#[cfg(any(feature = "sqlite", feature = "mysql", feature = "postgresql"))]
#[allow(unused_imports)]
pub(crate) use type_identifier::*;

pub use self::metrics::query;
