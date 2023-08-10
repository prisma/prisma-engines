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

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod metrics;

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

#[cfg(feature = "mssql")]
pub(crate) mod mssql_common;

#[cfg(feature = "mssql-connector")]
pub(crate) mod mssql;

#[cfg(feature = "mysql")]
pub(crate) mod mysql_common;

#[cfg(feature = "mysql-connector")]
pub(crate) mod mysql;

#[cfg(feature = "postgresql")]
pub(crate) mod postgres_common;

#[cfg(feature = "postgresql-connector")]
pub(crate) mod postgres;

#[cfg(feature = "sqlite")]
pub(crate) mod sqlite_common;

#[cfg(feature = "sqlite-connector")]
pub(crate) mod sqlite;

#[cfg(feature = "mysql")]
pub use self::mysql_common::*;

#[cfg(feature = "mysql-connector")]
pub use self::mysql::*;

#[cfg(feature = "postgresql")]
pub use self::postgres_common::*;

#[cfg(feature = "postgresql-connector")]
pub use self::postgres::*;

pub use self::result_set::*;
pub use connection_info::*;

#[cfg(feature = "mssql")]
pub use mssql_common::*;

#[cfg(feature = "mssql-connector")]
pub use mssql::*;

pub use queryable::*;

#[cfg(feature = "sqlite")]
pub use sqlite_common::*;

#[cfg(feature = "sqlite-connector")]
pub use sqlite::*;

pub use transaction::*;

#[cfg(any(
    feature = "sqlite-connector",
    feature = "mysql-connector",
    feature = "postgresql-connector"
))]
#[allow(unused_imports)]
pub(crate) use type_identifier::*;
