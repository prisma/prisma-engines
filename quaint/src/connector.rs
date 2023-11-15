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
#[cfg(any(feature = "mssql-native", feature = "postgresql-native", feature = "mysql-native"))]
mod timeout;
mod transaction;
mod type_identifier;

pub use self::result_set::*;
pub use connection_info::*;
pub use queryable::*;
pub use transaction::*;
#[cfg(any(feature = "mssql-native", feature = "postgresql-native", feature = "mysql-native"))]
#[allow(unused_imports)]
pub(crate) use type_identifier::*;

pub use self::metrics::query;

#[cfg(feature = "postgresql")]
pub(crate) mod postgres;
#[cfg(feature = "postgresql-native")]
pub use postgres::native::*;
#[cfg(feature = "postgresql")]
pub use postgres::*;

#[cfg(feature = "mysql")]
pub(crate) mod mysql;
#[cfg(feature = "mysql-native")]
pub use mysql::native::*;
#[cfg(feature = "mysql")]
pub use mysql::*;

#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;
#[cfg(feature = "sqlite-native")]
pub use sqlite::native::*;
#[cfg(feature = "sqlite")]
pub use sqlite::*;

#[cfg(feature = "mssql")]
pub(crate) mod mssql;
#[cfg(feature = "mssql-native")]
pub use mssql::native::*;
#[cfg(feature = "mssql")]
pub use mssql::*;
