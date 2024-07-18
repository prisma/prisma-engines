//! Definitions for the SQLite connector.
//! This module is not compatible with wasm32-* targets.
//! This module is only available with the `sqlite-native` feature.
mod conversion;
mod error;

use crate::connector::sqlite::params::SqliteParams;
use crate::connector::IsolationLevel;

pub use rusqlite::{params_from_iter, version as sqlite_version};

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use std::convert::TryFrom;
use tokio::sync::Mutex;

/// The underlying sqlite driver. Only available with the `expose-drivers` Cargo feature.
#[cfg(feature = "expose-drivers")]
pub use rusqlite;

/// A connector interface for the SQLite database
pub struct Sqlite {
    pub(crate) client: Mutex<rusqlite::Connection>,
}

impl TryFrom<&str> for Sqlite {
    type Error = Error;

    fn try_from(path: &str) -> crate::Result<Self> {
        let params = SqliteParams::try_from(path)?;
        let file_path = params.file_path;

        // Read about SQLite threading modes here: https://www.sqlite.org/threadsafe.html.
        // - "single-thread". In this mode, all mutexes are disabled and SQLite is unsafe to use in more than a single thread at once.
        // - "multi-thread". In this mode, SQLite can be safely used by multiple threads provided that no single database connection nor any
        //   object derived from database connection, such as a prepared statement, is used in two or more threads at the same time.
        // - "serialized". In serialized mode, API calls to affect or use any SQLite database connection or any object derived from such a
        //   database connection can be made safely from multiple threads. The effect on an individual object is the same as if the API calls
        //   had all been made in the same order from a single thread.
        //
        // `rusqlite` uses `SQLITE_OPEN_NO_MUTEX` by default, which means that the connection uses the "multi-thread" threading mode.

        let conn = rusqlite::Connection::open_with_flags(
            file_path.as_str(),
            // The database is opened for reading and writing if possible, or reading only if the file is write protected by the operating system.
            // The database is created if it does not already exist.
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                // The new database connection will use the "multi-thread" threading mode.
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                // The filename can be interpreted as a URI if this flag is set.
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?;

        if let Some(timeout) = params.socket_timeout {
            conn.busy_timeout(timeout)?;
        };

        let client = Mutex::new(conn);

        Ok(Sqlite { client })
    }
}

impl Sqlite {
    pub fn new(file_path: &str) -> crate::Result<Sqlite> {
        Self::try_from(file_path)
    }

    /// Open a new SQLite database in memory.
    pub fn new_in_memory() -> crate::Result<Sqlite> {
        let client = rusqlite::Connection::open_in_memory()?;

        Ok(Sqlite {
            client: Mutex::new(client),
        })
    }

    /// The underlying rusqlite::Connection. Only available with the `expose-drivers` Cargo
    /// feature. This is a lower level API when you need to get into database specific features.
    #[cfg(feature = "expose-drivers")]
    pub fn connection(&self) -> &Mutex<rusqlite::Connection> {
        &self.client
    }
}

impl_default_TransactionCapable!(Sqlite);

#[async_trait]
impl Queryable for Sqlite {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Sqlite::build(q)?;
        self.query_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("sqlite.query_raw", sql, params, move || async move {
            let client = self.client.lock().await;

            let mut stmt = client.prepare_cached(sql)?;

            let mut rows = stmt.query(params_from_iter(params.iter()))?;
            let mut result = ResultSet::new(rows.to_column_names(), Vec::new());

            while let Some(row) = rows.next()? {
                result.rows.push(row.get_result_row()?);
            }

            result.set_last_insert_id(u64::try_from(client.last_insert_rowid()).unwrap_or(0));

            Ok(result)
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Sqlite::build(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("sqlite.query_raw", sql, params, move || async move {
            let client = self.client.lock().await;
            let mut stmt = client.prepare_cached(sql)?;
            let res = u64::try_from(stmt.execute(params_from_iter(params.iter()))?)?;

            Ok(res)
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("sqlite.raw_cmd", cmd, &[], move || async move {
            let client = self.client.lock().await;
            client.execute_batch(cmd)?;
            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        Ok(Some(rusqlite::version().into()))
    }

    fn is_healthy(&self) -> bool {
        true
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        // SQLite is always "serializable", other modes involve pragmas
        // and shared cache mode, which is out of scope for now and should be implemented
        // as part of a separate effort.
        if !matches!(isolation_level, IsolationLevel::Serializable) {
            let kind = ErrorKind::invalid_isolation_level(&isolation_level);
            return Err(Error::builder(kind).build());
        }

        Ok(())
    }

    fn requires_isolation_first(&self) -> bool {
        false
    }

    fn begin_statement(&self) -> &'static str {
        // From https://sqlite.org/isolation.html:
        // `BEGIN IMMEDIATE` avoids possible `SQLITE_BUSY_SNAPSHOT` that arise when another connection jumps ahead in line.
        //  The BEGIN IMMEDIATE command goes ahead and starts a write transaction, and thus blocks all other writers.
        // If the BEGIN IMMEDIATE operation succeeds, then no subsequent operations in that transaction will ever fail with an SQLITE_BUSY error.
        "BEGIN IMMEDIATE"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::*,
        connector::Queryable,
        error::{ErrorKind, Name},
    };

    #[tokio::test]
    async fn unknown_table_should_give_a_good_error() {
        let conn = Sqlite::try_from("file:db/test.db").unwrap();
        let select = Select::from_table("not_there");

        let err = conn.select(select).await.unwrap_err();

        match err.kind() {
            ErrorKind::TableDoesNotExist { table } => {
                assert_eq!(&Name::available("not_there"), table);
            }
            e => panic!("Expected error TableDoesNotExist, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn in_memory_sqlite_works() {
        let conn = Sqlite::new_in_memory().unwrap();

        conn.raw_cmd("CREATE TABLE test (id INTEGER PRIMARY KEY, txt TEXT NOT NULL);")
            .await
            .unwrap();

        let insert = Insert::single_into("test").value("txt", "henlo");
        conn.insert(insert.into()).await.unwrap();

        let select = Select::from_table("test").value(asterisk());
        let result = conn.select(select.clone()).await.unwrap();
        let result = result.into_single().unwrap();

        assert_eq!(result.get("id").unwrap(), &Value::int32(1));
        assert_eq!(result.get("txt").unwrap(), &Value::text("henlo"));

        // Check that we do get a separate, new database.
        let other_conn = Sqlite::new_in_memory().unwrap();

        let err = other_conn.select(select).await.unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::TableDoesNotExist { .. }));
    }

    #[tokio::test]
    async fn quoting_in_returning_in_sqlite_works() {
        let conn = Sqlite::new_in_memory().unwrap();

        conn.raw_cmd("CREATE TABLE test (id  INTEGER PRIMARY KEY, `txt space` TEXT NOT NULL);")
            .await
            .unwrap();

        let insert = Insert::single_into("test").value("txt space", "henlo");
        conn.insert(insert.into()).await.unwrap();

        let select = Select::from_table("test").value(asterisk());
        let result = conn.select(select.clone()).await.unwrap();
        let result = result.into_single().unwrap();

        assert_eq!(result.get("id").unwrap(), &Value::int32(1));
        assert_eq!(result.get("txt space").unwrap(), &Value::text("henlo"));

        let insert = Insert::single_into("test").value("txt space", "henlo");
        let insert: Insert = Insert::from(insert).returning(["txt space"]);

        let result = conn.insert(insert).await.unwrap();
        let result = result.into_single().unwrap();

        assert_eq!(result.get("txt space").unwrap(), &Value::text("henlo"));
    }
}
