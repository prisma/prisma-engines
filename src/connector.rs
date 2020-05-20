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
#[cfg(feature = "sqlite")]
pub use sqlite::*;

mod connection_info;
pub use connection_info::*;

pub(crate) mod metrics;
pub use self::result_set::*;
pub use queryable::*;
pub use transaction::*;
