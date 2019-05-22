use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    transaction::{ResultRow, ToResultRow, Transaction, Transactional},
    visitor::{self, Visitor},
    QueryResult,
};
use libsqlite3_sys as ffi;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{
    types::{FromSqlError, ValueRef},
    Connection, Row as SqliteRow, Transaction as SqliteTransaction, NO_PARAMS,
};
use std::collections::HashSet;

type Pool = r2d2::Pool<SqliteConnectionManager>;

pub struct Sqlite {
    databases_folder_path: String,
    pool: Pool,
    test_mode: bool,
}

impl Transactional for Sqlite {
    fn with_transaction<F, T>(&self, db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Transaction) -> QueryResult<T>,
    {
        self.with_connection(db, |ref mut conn| {
            let mut tx = conn.transaction()?;
            tx.set_prepared_statement_cache_capacity(65536);

            let result = f(&mut tx);

            if result.is_ok() {
                tx.commit()?;
            }

            result
        })
    }
}

impl<'a> Transaction for SqliteTransaction<'a> {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        let (sql, params) = dbg!(visitor::Sqlite::build(q));

        let mut stmt = self.prepare_cached(&sql)?;
        stmt.execute(params)?;

        Ok(Some(Id::Int(self.last_insert_rowid() as usize)))
    }

    fn query(&mut self, q: Query) -> QueryResult<Vec<ResultRow>> {
        let (sql, params) = dbg!(visitor::Sqlite::build(q));

        let mut stmt = self.prepare_cached(&sql)?;
        let mut rows = stmt.query(params)?;
        let mut result = Vec::new();

        while let Some(row) = rows.next()? {
            result.push(row.to_result_row()?);
        }

        Ok(result)
    }
}

impl<'a> ToResultRow for SqliteRow<'a> {
    fn to_result_row<'b>(&'b self) -> QueryResult<ResultRow> {
        let mut row = ResultRow::default();

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
                ValueRef::Text(s) => ParameterizedValue::Text(s.to_string()),
                ValueRef::Blob(_) => panic!("Blobs not supprted, yet"),
            };

            row.values.push(pv);
        }

        Ok(row)
    }
}

impl Sqlite {
    pub fn new(
        databases_folder_path: String,
        connection_limit: u32,
        test_mode: bool,
    ) -> QueryResult<Sqlite> {
        let pool = r2d2::Pool::builder()
            .max_size(connection_limit)
            .build(SqliteConnectionManager::memory())?;

        Ok(Sqlite {
            databases_folder_path,
            pool,
            test_mode,
        })
    }

    fn attach_database(&self, conn: &mut Connection, db_name: &str) -> QueryResult<()> {
        let mut stmt = conn.prepare("PRAGMA database_list")?;

        let databases: HashSet<String> = stmt
            .query_map(NO_PARAMS, |row| {
                let name: String = row.get(1)?;

                Ok(name)
            })?
            .map(|res| res.unwrap())
            .collect();

        if !databases.contains(db_name) {
            // This is basically hacked until we have a full rust stack with a migration engine.
            // Currently, the scala tests use the JNA library to write to the database. This
            let database_file_path = format!("{}/{}.db", self.databases_folder_path, db_name);
            conn.execute(
                "ATTACH DATABASE ? AS ?",
                &[database_file_path.as_ref(), db_name],
            )?;
        }

        conn.execute("PRAGMA foreign_keys = ON", NO_PARAMS)?;
        Ok(())
    }

    fn with_connection<F, T>(&self, db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Connection) -> QueryResult<T>,
    {
        let mut conn = self.pool.get()?;
        self.attach_database(&mut conn, db)?;

        let result = f(&mut conn);

        if self.test_mode {
            conn.execute("DETACH DATABASE ?", &[db])?;
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

            e => Error::QueryError(e.into()),
        }
    }
}

impl From<FromSqlError> for Error {
    fn from(e: FromSqlError) -> Error {
        Error::ColumnReadFailure(e.into())
    }
}
