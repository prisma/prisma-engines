use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    transaction::{
        ColumnNames, Connection, Connectional, Row, ToColumnNames, ToRow, Transaction,
        Transactional,
    },
    visitor::{self, Visitor},
    ResultSet,
};
use libsqlite3_sys as ffi;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{
    types::{FromSqlError, ValueRef},
    Connection as SqliteConnection, Row as SqliteRow, Rows as SqliteRows,
    Transaction as SqliteTransaction, NO_PARAMS,
};
use std::{collections::HashSet, convert::TryFrom, path::PathBuf};

type PooledConnection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;
type Pool = r2d2::Pool<SqliteConnectionManager>;

pub struct Sqlite {
    file_path: String,
    pool: Pool,
    test_mode: bool,
}

impl Transactional for Sqlite {
    type Error = Error;

    fn with_transaction<F, T>(&self, db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Transaction) -> crate::Result<T>,
    {
        self.with_connection_internal(db, |ref mut conn| {
            let mut tx = conn.get_mut().transaction()?;
            tx.set_prepared_statement_cache_capacity(65536);

            let result = f(&mut tx);

            if result.is_ok() {
                tx.commit()?;
            }

            result
        })
    }
}

impl Connectional for Sqlite {
    fn with_connection<'a, F, T>(&self, db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Connection) -> crate::Result<T>,
        Self: Sized,
    {
        self.with_connection_internal(db, |c| f(c.get_mut()))
    }

    fn execute_on_connection<'a>(&self, db: &str, query: Query<'a>) -> crate::Result<Option<Id>> {
        self.with_connection(&db, |conn| conn.execute(query))
    }

    fn query_on_connection<'a>(&self, db: &str, query: Query<'a>) -> crate::Result<ResultSet> {
        self.with_connection(&db, |conn| conn.query(query))
    }

    fn query_on_raw_connection<'a>(
        &self,
        db: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        self.with_connection(&db, |conn| conn.query_raw(&sql, &params))
    }
}

// Concrete implmentations of trait methods, dropping the mut
// so we can share it between Connection and Transaction.
fn execute_impl<'a>(conn: &SqliteConnection, q: Query<'a>) -> crate::Result<Option<Id>> {
    let (sql, params) = dbg!(visitor::Sqlite::build(q));

    let mut stmt = conn.prepare_cached(&sql)?;
    stmt.execute(params)?;

    Ok(Some(Id::Int(conn.last_insert_rowid() as usize)))
}

fn query_impl<'a>(conn: &SqliteConnection, q: Query<'a>) -> crate::Result<ResultSet> {
    let (sql, params) = dbg!(visitor::Sqlite::build(q));

    return query_raw_impl(conn, &sql, &params);
}

fn query_raw_impl<'a>(
    conn: &SqliteConnection,
    sql: &str,
    params: &[ParameterizedValue<'a>],
) -> crate::Result<ResultSet> {
    let mut stmt = conn.prepare_cached(sql)?;
    let mut rows = stmt.query(params)?;

    let mut result = ResultSet::new(rows.to_column_names(), Vec::new());

    while let Some(row) = rows.next()? {
        result.rows.push(row.to_result_row()?);
    }

    Ok(result)
}

// Exploits that sqlite::Transaction implements std::ops::Deref<&sqlite::Connection>.
// Dereferenced Connection is immuteable!
impl<'a> Transaction for SqliteTransaction<'a> {}

// Trait implementation for r2d2 pooled connection.
impl Connection for PooledConnection {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        execute_impl(self, q)
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        query_impl(self, q)
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        query_raw_impl(self, sql, params)
    }
}

// Trait implementation for r2d2 sqlite.
impl<'t> Connection for SqliteTransaction<'t> {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        execute_impl(self, q)
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        query_impl(self, q)
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        query_raw_impl(self, sql, params)
    }
}

impl Connection for SqliteConnection {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        execute_impl(self, q)
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        query_impl(self, q)
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        query_raw_impl(self, sql, params)
    }
}

impl<'a> ToRow for SqliteRow<'a> {
    fn to_result_row<'b>(&'b self) -> crate::Result<Row> {
        let mut row = Row::default();

        for (i, column) in self.columns().iter().enumerate() {
            let pv = match self.get_raw(i) {
                ValueRef::Null => ParameterizedValue::Null,
                ValueRef::Integer(i) => match column.decl_type() {
                    Some("BOOLEAN") => {
                        if i == 0 {
                            ParameterizedValue::Boolean(false)
                        } else {
                            ParameterizedValue::Boolean(true)
                        }
                    }
                    _ => ParameterizedValue::Integer(i),
                },
                ValueRef::Real(f) => ParameterizedValue::Real(f),
                ValueRef::Text(s) => ParameterizedValue::Text(s.to_string().into()),
                ValueRef::Blob(_) => panic!("Blobs not supprted, yet"),
            };

            row.values.push(pv);
        }

        Ok(row)
    }
}

impl<'a> ToColumnNames for SqliteRows<'a> {
    fn to_column_names<'b>(&'b self) -> ColumnNames {
        let mut names = ColumnNames::default();

        if let Some(columns) = self.column_names() {
            for column in columns {
                names.names.push(String::from(column));
            }
        }

        names
    }
}

impl TryFrom<&str> for Sqlite {
    type Error = Error;

    /// Todo connection limit configuration
    fn try_from(url: &str) -> crate::Result<Sqlite> {
        // We must handle file URLs ourselves.
        let normalized = url.trim_start_matches("file:");
        let path = PathBuf::from(&normalized);

        if path.is_dir() {
            Err(Error::DatabaseUrlIsInvalid(url.to_string()))
        } else {
            Sqlite::new(normalized.to_string(), 10, false)
        }
    }
}

impl Sqlite {
    pub fn new(file_path: String, connection_limit: u32, test_mode: bool) -> crate::Result<Sqlite> {
        let pool = r2d2::Pool::builder()
            .max_size(connection_limit)
            .build(SqliteConnectionManager::memory())?;

        Ok(Sqlite {
            file_path,
            pool,
            test_mode,
        })
    }

    pub fn does_file_exist(&self) -> bool {
        let path = PathBuf::from(&self.file_path);
        path.exists()
    }

    fn attach_database(&self, conn: &mut SqliteConnection, db_name: &str) -> crate::Result<()> {
        let mut stmt = conn.prepare("PRAGMA database_list")?;

        let databases: HashSet<String> = stmt
            .query_map(NO_PARAMS, |row| {
                let name: String = row.get(1)?;

                Ok(name)
            })?
            .map(|res| res.unwrap())
            .collect();

        if !databases.contains(db_name) {
            SqliteConnection::execute(
                conn,
                "ATTACH DATABASE ? AS ?",
                &[self.file_path.as_ref(), db_name],
            )?;
        }

        SqliteConnection::execute(conn, "PRAGMA foreign_keys = ON", NO_PARAMS)?;
        Ok(())
    }

    fn with_connection_internal<F, T>(&self, db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut std::cell::RefCell<PooledConnection>) -> crate::Result<T>,
    {
        let mut conn = std::cell::RefCell::new(self.pool.get()?);
        self.attach_database(conn.get_mut(), db)?;

        let result = f(&mut conn);

        if self.test_mode {
            SqliteConnection::execute(conn.get_mut(), "DETACH DATABASE ?", &[db])?;
        }

        result
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound,

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 2067,
                },
                Some(description),
            ) => {
                let splitted: Vec<&str> = description.split(": ").collect();
                let splitted: Vec<&str> = splitted[1].split(".").collect();

                Error::UniqueConstraintViolation {
                    field_name: splitted[1].into(),
                }
            }

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 1555,
                },
                Some(description),
            ) => {
                let splitted: Vec<&str> = description.split(": ").collect();
                let splitted: Vec<&str> = splitted[1].split(".").collect();

                Error::UniqueConstraintViolation {
                    field_name: splitted[1].into(),
                }
            }

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 1299,
                },
                Some(description),
            ) => {
                let splitted: Vec<&str> = description.split(": ").collect();
                let splitted: Vec<&str> = splitted[1].split(".").collect();

                Error::NullConstraintViolation {
                    field_name: splitted[1].into(),
                }
            }

            e => Error::QueryError(e.into()),
        }
    }
}

impl From<FromSqlError> for Error {
    fn from(e: FromSqlError) -> Error {
        Error::ColumnReadFailure(e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_provide_a_database_connection() {
        let connector = Sqlite::new(String::from("db/test.db"), 1, true).unwrap();

        connector
            .with_connection("test", |connection| {
                let res = connection.query_raw("SELECT * FROM sqlite_master", &[])?;

                // No results expected.
                assert!(res.is_empty());

                Ok(())
            })
            .unwrap()
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let connector = Sqlite::new(String::from("db/test.db"), 1, true).unwrap();

        connector
            .with_transaction("test", |transaction| {
                let res = transaction.query_raw("SELECT * FROM sqlite_master", &[])?;

                // No results expected.
                assert!(res.is_empty());

                Ok(())
            })
            .unwrap()
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

    #[test]
    fn should_map_columns_correctly() {
        let connector = Sqlite::new(String::from("db/test.db"), 1, true).unwrap();

        connector
            .with_connection("test", |connection| {
                connection.query_raw(TABLE_DEF, &[])?;
                connection.query_raw(CREATE_USER, &[])?;

                let rows = connection.query_raw("SELECT * FROM USER", &[])?;
                assert_eq!(rows.len(), 1);

                let row = rows.get(0).unwrap();
                assert_eq!(row["ID"].as_i64(), Some(1));
                assert_eq!(row["NAME"].as_str(), Some("Joe"));
                assert_eq!(row["AGE"].as_i64(), Some(27));
                assert_eq!(row["SALARY"].as_f64(), Some(20000.0));

                Ok(())
            })
            .unwrap()
    }
}
