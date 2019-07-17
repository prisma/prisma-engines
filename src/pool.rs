//! Support for the `r2d2` connection pool.

#[cfg(feature = "mysql-16")]
pub mod mysql;

#[cfg(feature = "postgresql-0_16")]
pub mod postgres;

#[cfg(feature = "rusqlite-0_19")]
pub mod sqlite;

use std::path::PathBuf;

/// An `r2d2::ManageConnection` for all of the connectors supported by
/// prisma-query.
///
/// ## Sqlite
///
/// ```no_run
/// use prisma_query::connector::Queryable;
/// use prisma_query::ast::*;
/// use prisma_query::pool::PrismaConnectionManager;
/// use std::thread;
///
/// fn main() {
///     let manager = PrismaConnectionManager::new("db/test.db").unwrap();
///     let pool = r2d2::Pool::new(manager).unwrap();
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         thread::spawn(move || {
///             let mut client = pool.get().unwrap();
///             let insert = Insert::single_into("foo").value("key", i);
///
///             client.execute(Query::from(insert)).unwrap();
///         });
///     }
/// }
/// ```
///
/// ## PostgreSQL
///
/// ```no_run
/// use prisma_query::connector::Queryable;
/// use prisma_query::ast::*;
/// use prisma_query::pool::PrismaConnectionManager;
/// use postgres::Client;
/// use std::{thread, convert::TryFrom};
///
/// fn main() {
///     let mut config = Client::configure();
///     config.host("localhost");
///     config.user("root");
///
///     let manager = PrismaConnectionManager::try_from(config).unwrap();
///     let pool = r2d2::Pool::new(manager).unwrap();
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         thread::spawn(move || {
///             let mut client = pool.get().unwrap();
///             let insert = Insert::single_into("foo").value("key", i);
///
///             client.execute(Query::from(insert)).unwrap();
///         });
///     }
/// }
/// ```
///
/// ## MySQL
///
/// ```no_run
/// use prisma_query::connector::Queryable;
/// use prisma_query::ast::*;
/// use prisma_query::pool::PrismaConnectionManager;
/// use mysql::OptsBuilder;
/// use std::{thread, convert::TryFrom};
///
/// fn main() {
///     let mut opts = OptsBuilder::new();
///     opts.ip_or_hostname(Some("localhost"));
///     opts.user(Some("root"));
///
///     let manager = PrismaConnectionManager::from(opts);
///     let pool = r2d2::Pool::new(manager).unwrap();
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         thread::spawn(move || {
///             let mut client = pool.get().unwrap();
///             let insert = Insert::single_into("foo").value("key", i);
///
///             client.execute(Query::from(insert)).unwrap();
///         });
///     }
/// }
/// ```
pub struct PrismaConnectionManager<Inner>
where
    Inner: r2d2::ManageConnection,
{
    inner: Inner,
    file_path: Option<PathBuf>,
}
