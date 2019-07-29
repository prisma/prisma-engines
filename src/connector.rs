//! A set of abstractions for database connections.
//!
//! Provides traits for database querying and executing, and for spawning
//! transactions.
//!
//! Connectors for [MySQL](struct.Mysql.html),
//! [PostgreSQL](struct.PostgreSql.html) and [SQLite](struct.Sqlite.html) connect
//! to the corresponding databases and implement the
//! [Queryable](trait.Queryable.html) trait for generalized querying interface.
mod queryable;
mod result_set;
mod transaction;

#[cfg(feature = "mysql-16")]
pub(crate) mod mysql;

#[cfg(feature = "postgresql-0_16")]
pub(crate) mod postgres;

#[cfg(feature = "rusqlite-0_19")]
pub(crate) mod sqlite;

#[cfg(feature = "mysql-16")]
pub use self::mysql::*;

#[cfg(feature = "postgresql-0_16")]
pub use self::postgres::PostgreSql;

#[cfg(feature = "rusqlite-0_19")]
pub use sqlite::*;

pub use self::result_set::*;
pub use queryable::*;
pub use transaction::*;
