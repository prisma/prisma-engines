mod conversion;
mod error;

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

pub struct SqliteParams {
    pub connection_limit: Option<usize>,
    /// This is not a `PathBuf` because we need to `ATTACH` the database to the path, and this can
    /// only be done with UTF-8 paths.
    pub file_path: String,
    pub db_name: String,
    pub socket_timeout: Option<Duration>,
}

type ConnectionParams = (Vec<(String, String)>, Vec<(String, String)>);

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
            let official = vec![];
            let mut connection_limit = None;
            let mut db_name = None;
            let mut socket_timeout = None;

            if path_parts.len() > 1 {
                let (_, unsupported): ConnectionParams = path_parts
                    .last()
                    .unwrap()
                    .split('&')
                    .map(|kv| {
                        let splitted: Vec<&str> = kv.split('=').collect();
                        (String::from(splitted[0]), String::from(splitted[1]))
                    })
                    .collect::<Vec<(String, String)>>()
                    .into_iter()
                    .partition(|(k, _)| official.contains(&k.as_str()));

                for (k, v) in unsupported.into_iter() {
                    match k.as_ref() {
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
                            tracing::trace!(message = "Discarding connection string param", param = k.as_str());
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
        let (sql, params) = visitor::Sqlite::build(q);
        self.query_raw(&sql, &params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Sqlite::build(q);
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
    use crate::{
        ast::*,
        col,
        connector::{Queryable, TransactionCapable},
        error::{DatabaseConstraint, ErrorKind},
        single::Quaint,
        val, values,
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
    async fn should_provide_a_database_connection() {
        let connection = Sqlite::new("db/test.db").unwrap();
        let res = connection.query_raw("SELECT * FROM sqlite_master", &[]).await.unwrap();

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn should_provide_a_database_transaction() {
        let connection = Sqlite::new("db/test.db").unwrap();
        let tx = connection.start_transaction().await.unwrap();
        let res = tx.query_raw("SELECT * FROM sqlite_master", &[]).await.unwrap();

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_aliased_value() {
        let conn = Sqlite::new("db/test.db").unwrap();
        let query = Select::default().value(val!(1).alias("test"));
        let rows = conn.select(query).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(Value::Integer(1), row["test"]);
    }

    #[tokio::test]
    async fn test_aliased_null() {
        let conn = Sqlite::new("db/test.db").unwrap();
        let query = Select::default().value(val!(Option::<i64>::None).alias("test"));
        let rows = conn.select(query).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(Value::Null, row["test"]);
    }

    #[tokio::test]
    async fn tuples_in_selection() {
        let table = r#"
            CREATE TABLE tuples (id SERIAL PRIMARY KEY, age INTEGER NOT NULL, length REAL NOT NULL);
        "#;

        let connection = Quaint::new("file:db/test.db").await.unwrap();

        connection.query_raw("DROP TABLE IF EXISTS tuples", &[]).await.unwrap();
        connection.query_raw(table, &[]).await.unwrap();

        let insert = Insert::multi_into("tuples", vec!["age", "length"])
            .values(vec![val!(35), val!(20.0)])
            .values(vec![val!(40), val!(18.0)]);

        connection.insert(insert.into()).await.unwrap();

        // 1-tuple
        {
            let mut cols = Row::new();
            cols.push(Column::from("age"));

            let mut vals = Row::new();
            vals.push(35);

            let select = Select::from_table("tuples").so_that(cols.in_selection(vals));
            let rows = connection.select(select).await.unwrap();

            let row = rows.get(0).unwrap();
            assert_eq!(row["age"].as_i64(), Some(35));
            assert_eq!(row["length"].as_f64(), Some(20.0));
        }

        // 2-tuple
        {
            let cols = Row::from((col!("age"), col!("length")));
            let vals = values!((35, 20.0));

            let select = Select::from_table("tuples").so_that(cols.in_selection(vals));
            let rows = connection.select(select).await.unwrap();

            let row = rows.get(0).unwrap();
            assert_eq!(row["age"].as_i64(), Some(35));
            assert_eq!(row["length"].as_f64(), Some(20.0));
        }
    }

    #[allow(unused)]
    const TABLE_DEF: &str = r#"
    CREATE TABLE USER (
        ID INT PRIMARY KEY     NOT NULL,
        NAME           TEXT    NOT NULL,
        AGE            INT     NOT NULL,
        SALARY         REAL
    );
    "#;

    #[allow(unused)]
    const CREATE_USER: &str = r#"
    INSERT INTO USER (ID,NAME,AGE,SALARY)
    VALUES (1, 'Joe', 27, 20000.00 );
    "#;

    #[tokio::test]
    async fn should_map_columns_correctly() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();

        connection.query_raw(TABLE_DEF, &[]).await.unwrap();

        let changes = connection.execute_raw(CREATE_USER, &[]).await.unwrap();
        assert_eq!(1, changes);

        let rows = connection.query_raw("SELECT * FROM USER", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["ID"].as_i64(), Some(1));
        assert_eq!(row["NAME"].as_str(), Some("Joe"));
        assert_eq!(row["AGE"].as_i64(), Some(27));
        assert_eq!(row["SALARY"].as_f64(), Some(20000.0));
    }

    #[tokio::test]
    async fn op_test_add_one_level() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(2) + val!(1));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(3));
    }

    #[tokio::test]
    async fn op_test_add_two_levels() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(2) + val!(val!(3) + val!(2)));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(7));
    }

    #[tokio::test]
    async fn op_test_sub_one_level() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(2) - val!(1));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(1));
    }

    #[tokio::test]
    async fn op_test_sub_three_items() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(2) - val!(1) - val!(1));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(0));
    }

    #[tokio::test]
    async fn op_test_sub_two_levels() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(2) - val!(val!(3) + val!(1)));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(-2));
    }

    #[tokio::test]
    async fn op_test_mul_one_level() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(6) * val!(6));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(36));
    }

    #[tokio::test]
    async fn op_test_mul_two_levels() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(6) * (val!(6) - val!(1)));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(30));
    }

    #[tokio::test]
    async fn op_multiple_operations() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(4) - val!(2) * val!(2));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(0));
    }

    #[tokio::test]
    async fn op_test_div_one_level() {
        let connection = Sqlite::try_from("file:db/test.db").unwrap();
        let q = Select::default().value(val!(6) / val!(3));

        let rows = connection.select(q).await.unwrap();
        let row = rows.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(2));
    }

    #[tokio::test]
    async fn test_uniq_constraint_violation() {
        let conn = Sqlite::try_from("file:db/test.db").unwrap();

        let _ = conn.raw_cmd("DROP TABLE test_uniq_constraint_violation").await;

        conn.raw_cmd("CREATE TABLE test_uniq_constraint_violation (id1 int, id2 int)")
            .await
            .unwrap();
        conn.raw_cmd("CREATE UNIQUE INDEX musti ON test_uniq_constraint_violation (id1, id2)")
            .await
            .unwrap();

        conn.query_raw(
            "INSERT INTO test_uniq_constraint_violation (id1, id2) VALUES (1, 2)",
            &[],
        )
        .await
        .unwrap();

        let res = conn
            .query_raw(
                "INSERT INTO test_uniq_constraint_violation (id1, id2) VALUES (1, 2)",
                &[],
            )
            .await;

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::UniqueConstraintViolation { constraint } => {
                assert_eq!(Some("2067"), err.original_code());
                assert_eq!(Some("UNIQUE constraint failed: test_uniq_constraint_violation.id1, test_uniq_constraint_violation.id2"), err.original_message());

                assert_eq!(
                    &DatabaseConstraint::Fields(vec![String::from("id1"), String::from("id2")]),
                    constraint,
                )
            }
            _ => panic!(err),
        }
    }

    #[tokio::test]
    async fn test_null_constraint_violation() {
        let conn = Sqlite::try_from("file:db/test.db").unwrap();

        let _ = conn.raw_cmd("DROP TABLE test_null_constraint_violation").await;

        conn.raw_cmd("CREATE TABLE test_null_constraint_violation (id1 int not null, id2 int not null)")
            .await
            .unwrap();

        let res = conn
            .query_raw("INSERT INTO test_null_constraint_violation DEFAULT VALUES", &[])
            .await;

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::NullConstraintViolation { constraint } => {
                assert_eq!(Some("1299"), err.original_code());
                assert_eq!(
                    Some("NOT NULL constraint failed: test_null_constraint_violation.id1"),
                    err.original_message()
                );
                assert_eq!(&DatabaseConstraint::Fields(vec![String::from("id1")]), constraint)
            }
            _ => panic!(err),
        }
    }
}
