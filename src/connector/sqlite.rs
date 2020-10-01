mod conversion;
mod error;

pub use rusqlite::version as sqlite_version;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use rusqlite::NO_PARAMS;
use std::{collections::HashSet, convert::TryFrom, path::Path, time::Duration};
use tokio::sync::Mutex;

const DEFAULT_SCHEMA_NAME: &str = "quaint";

/// A connector interface for the SQLite database
pub struct Sqlite {
    pub(crate) client: Mutex<rusqlite::Connection>,
    /// This is not a `PathBuf` because we need to `ATTACH` the database to the path, and this can
    /// only be done with UTF-8 paths.
    pub(crate) file_path: String,
}

#[derive(Debug)]
pub struct SqliteParams {
    pub connection_limit: Option<usize>,
    /// This is not a `PathBuf` because we need to `ATTACH` the database to the path, and this can
    /// only be done with UTF-8 paths.
    pub file_path: String,
    pub db_name: String,
    pub socket_timeout: Option<Duration>,
}

impl TryFrom<&str> for SqliteParams {
    type Error = Error;

    fn try_from(path: &str) -> crate::Result<Self> {
        let path = if path.starts_with("file:") {
            path.trim_start_matches("file:")
        } else {
            path.trim_start_matches("sqlite:")
        };

        let path_parts: Vec<&str> = path.split('?').collect();
        let path_str = path_parts[0];
        let path = Path::new(path_str);

        if path.is_dir() {
            Err(Error::builder(ErrorKind::DatabaseUrlIsInvalid(path.to_str().unwrap().to_string())).build())
        } else {
            let mut connection_limit = None;
            let mut db_name = None;
            let mut socket_timeout = None;

            if path_parts.len() > 1 {
                let params = path_parts.last().unwrap().split('&').map(|kv| {
                    let splitted: Vec<&str> = kv.split('=').collect();
                    (splitted[0], splitted[1])
                });

                for (k, v) in params {
                    match k {
                        "connection_limit" => {
                            let as_int: usize = v
                                .parse()
                                .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                            connection_limit = Some(as_int);
                        }
                        "db_name" => {
                            db_name = Some(v.to_string());
                        }
                        "socket_timeout" => {
                            let as_int = v
                                .parse()
                                .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                            socket_timeout = Some(Duration::from_secs(as_int));
                        }
                        _ => {
                            #[cfg(not(feature = "tracing-log"))]
                            trace!("Discarding connection string param: {}", k);
                            #[cfg(feature = "tracing-log")]
                            tracing::trace!(message = "Discarding connection string param", param = k);
                        }
                    };
                }
            }

            Ok(Self {
                connection_limit,
                file_path: path_str.to_owned(),
                db_name: db_name.unwrap_or_else(|| DEFAULT_SCHEMA_NAME.to_owned()),
                socket_timeout,
            })
        }
    }
}

impl TryFrom<&str> for Sqlite {
    type Error = Error;

    fn try_from(path: &str) -> crate::Result<Self> {
        let params = SqliteParams::try_from(path)?;

        let conn = rusqlite::Connection::open_in_memory()?;

        if let Some(timeout) = params.socket_timeout {
            conn.busy_timeout(timeout)?;
        };

        let client = Mutex::new(conn);
        let file_path = params.file_path;

        Ok(Sqlite { client, file_path })
    }
}

impl Sqlite {
    pub fn new(file_path: &str) -> crate::Result<Sqlite> {
        Self::try_from(file_path)
    }

    pub async fn attach_database(&mut self, db_name: &str) -> crate::Result<()> {
        let client = self.client.lock().await;
        let mut stmt = client.prepare("PRAGMA database_list")?;

        let databases: HashSet<String> = stmt
            .query_map(NO_PARAMS, |row| {
                let name: String = row.get(1)?;

                Ok(name)
            })?
            .map(|res| res.unwrap())
            .collect();

        if !databases.contains(db_name) {
            rusqlite::Connection::execute(&client, "ATTACH DATABASE ? AS ?", &[self.file_path.as_str(), db_name])?;
        }

        rusqlite::Connection::execute(&client, "PRAGMA foreign_keys = ON", NO_PARAMS)?;

        Ok(())
    }
}

impl TransactionCapable for Sqlite {}

#[async_trait]
impl Queryable for Sqlite {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Sqlite::build(q)?;
        self.query_raw(&sql, &params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Sqlite::build(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("sqlite.query_raw", sql, params, move || async move {
            let client = self.client.lock().await;

            let mut stmt = client.prepare_cached(sql)?;

            let mut rows = stmt.query(params)?;
            let mut result = ResultSet::new(rows.to_column_names(), Vec::new());

            while let Some(row) = rows.next()? {
                result.rows.push(row.get_result_row()?);
            }

            result.set_last_insert_id(u64::try_from(client.last_insert_rowid()).unwrap_or(0));

            Ok(result)
        })
        .await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("sqlite.query_raw", sql, params, move || async move {
            let client = self.client.lock().await;
            let mut stmt = client.prepare_cached(sql)?;
            let res = u64::try_from(stmt.execute(params)?)?;

            Ok(res)
        })
        .await
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::*, connector::Queryable, error::ErrorKind};

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
                assert_eq!("not_there", table.as_str());
            }
            e => panic!("Expected error TableDoesNotExist, got {:?}", e),
        }
    }
}
