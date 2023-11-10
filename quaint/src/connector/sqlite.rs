mod conversion;
mod error;

pub(crate) use super::sqlite_wasm::{SqliteParams, DEFAULT_SQLITE_SCHEMA_NAME};
pub use error::SqliteError;

pub use rusqlite::{params_from_iter, version as sqlite_version};

use super::IsolationLevel;
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

        let conn = rusqlite::Connection::open(file_path.as_str())?;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::*,
        connector::Queryable,
        error::{ErrorKind, Name},
    };

    #[test]
    fn sqlite_params_from_str_should_resolve_path_correctly_with_file_scheme() {
        let path = "file:dev.db";
        let params = SqliteParams::try_from(path).unwrap();
        assert_eq!(params.file_path, "dev.db");
    }

    #[test]
    fn sqlite_params_from_str_should_resolve_path_correctly_with_sqlite_scheme() {
        let path = "sqlite:dev.db";
        let params = SqliteParams::try_from(path).unwrap();
        assert_eq!(params.file_path, "dev.db");
    }

    #[test]
    fn sqlite_params_from_str_should_resolve_path_correctly_with_no_scheme() {
        let path = "dev.db";
        let params = SqliteParams::try_from(path).unwrap();
        assert_eq!(params.file_path, "dev.db");
    }

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
