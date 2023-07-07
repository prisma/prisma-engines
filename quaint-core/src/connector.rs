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

mod queryable;
mod result_set;
mod transaction;
mod type_identifier;

pub use self::result_set::*;
pub use queryable::*;
pub use transaction::*;
// #[cfg(any(feature = "sqlite", feature = "mysql", feature = "postgresql"))]
// #[allow(unused_imports)]
pub(crate) use type_identifier::*;
