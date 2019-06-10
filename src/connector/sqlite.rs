use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    transaction::{
        ColumnNames, Connection, Connectional, ResultRow, ToColumnNames, ToResultRow, Transaction,
        Transactional,
    },
    visitor::{self, Visitor},
    QueryResult, ResultSet,
};
use libsqlite3_sys as ffi;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{
    types::{FromSqlError, ValueRef},
    Connection as SqliteConnection, Row as SqliteRow, Rows as SqliteRows,
    Transaction as SqliteTransaction, NO_PARAMS,
};
use std::collections::HashSet;

type PooledConnection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;
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
    fn with_connection<F, T>(&self, db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Connection) -> QueryResult<T>,
    {
        self.with_connection_internal(db, |c| f(c.get_mut()))
    }
}

// Concrete implmentations of trait methods, dropping the mut
// so we can share it between Connection and Transaction.
fn execute_impl(conn: &SqliteConnection, q: Query) -> QueryResult<Option<Id>> {
    let (sql, params) = dbg!(visitor::Sqlite::build(q));

    let mut stmt = conn.prepare_cached(&sql)?;
    stmt.execute(params)?;

    Ok(Some(Id::Int(conn.last_insert_rowid() as usize)))
}

fn query_impl(conn: &SqliteConnection, q: Query) -> QueryResult<ResultSet> {
    let (sql, params) = dbg!(visitor::Sqlite::build(q));

    return query_raw_impl(conn, &sql, &params);
}

fn query_raw_impl(
    conn: &SqliteConnection,
    sql: &str,
    params: &[ParameterizedValue],
) -> QueryResult<ResultSet> {
    let mut stmt = conn.prepare_cached(sql)?;
    let mut rows = stmt.query(params)?;

    let mut result = ResultSet::new(&rows.to_column_names(), Vec::new());

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
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        execute_impl(self, q)
    }

    fn query(&mut self, q: Query) -> QueryResult<ResultSet> {
        query_impl(self, q)
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> QueryResult<ResultSet> {
        query_raw_impl(self, sql, params)
    }
}

// Trait implementation for r2d2 sqlite.
impl<'a> Connection for SqliteTransaction<'a> {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        execute_impl(self, q)
    }

    fn query(&mut self, q: Query) -> QueryResult<ResultSet> {
        query_impl(self, q)
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> QueryResult<ResultSet> {
        query_raw_impl(self, sql, params)
    }
}

impl Connection for SqliteConnection {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        execute_impl(self, q)
    }

    fn query(&mut self, q: Query) -> QueryResult<ResultSet> {
        query_impl(self, q)
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> QueryResult<ResultSet> {
        query_raw_impl(self, sql, params)
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

    fn attach_database(&self, conn: &mut SqliteConnection, db_name: &str) -> QueryResult<()> {
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
            SqliteConnection::execute(
                conn,
                "ATTACH DATABASE ? AS ?",
                &[database_file_path.as_ref(), db_name],
            )?;
        }

        SqliteConnection::execute(conn, "PRAGMA foreign_keys = ON", NO_PARAMS)?;
        Ok(())
    }

    fn with_connection_internal<F, T>(&self, db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut std::cell::RefCell<PooledConnection>) -> QueryResult<T>,
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
    use crate::{connector::Sqlite, Connectional, Transactional};

    #[test]
    fn should_provide_a_database_connection() {
        let connector = Sqlite::new(String::from("db"), 1, true).unwrap();

        connector
            .with_connection("TEST", |connection| {
                let res = connection.query_raw("SELECT * FROM sqlite_master", &[])?;

                // No results expected.
                assert_eq!(res.into_iter().next().is_none(), true);

                Ok(())
            })
            .unwrap()
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let connector = Sqlite::new(String::from("db"), 1, true).unwrap();

        connector
            .with_transaction("TEST", |transaction| {
                let res = transaction.query_raw("SELECT * FROM sqlite_master", &[])?;

                // No results expected.
                assert_eq!(res.into_iter().next().is_none(), true);

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
        let connector = Sqlite::new(String::from("db"), 1, true).unwrap();

        connector
            .with_connection("TEST", |connection| {
                connection.query_raw(TABLE_DEF, &[])?;
                connection.query_raw(CREATE_USER, &[])?;

                let res = connection.query_raw("SELECT * FROM USER", &[])?;

                let mut result_count: u32 = 0;

                // Exactly one result expected.
                for row in &res {
                    assert_eq!(row.get_as_integer("ID")?, 1);
                    assert_eq!(row.get_as_string("NAME")?, "Joe");
                    assert_eq!(row.get_as_integer("AGE")?, 27);
                    assert_eq!(row.get_as_real("SALARY")?, 20000.0);
                    result_count = result_count + 1;
                }

                assert_eq!(result_count, 1);

                Ok(())
            })
            .unwrap()
    }
}
