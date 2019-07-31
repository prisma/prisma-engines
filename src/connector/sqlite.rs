mod conversion;
mod error;

use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet, Transaction},
    error::Error,
    visitor::{self, Visitor},
};
use rusqlite::NO_PARAMS;
use std::{collections::HashSet, convert::TryFrom, path::PathBuf};

/// A connector interface for the SQLite database
pub struct Sqlite {
    pub(crate) client: rusqlite::Connection,
    pub(crate) file_path: PathBuf,
}

pub struct SqliteParams {
    pub connection_limit: u32,
    pub file_path: PathBuf,
}

type ConnectionParams = (Vec<(String, String)>, Vec<(String, String)>);

impl TryFrom<&str> for SqliteParams {
    type Error = Error;

    fn try_from(path: &str) -> crate::Result<Self> {
        let path = path.trim_start_matches("file:");
        let path_parts: Vec<&str> = path.split("?").collect();

        let path = PathBuf::from(path_parts[0]);

        if path.is_dir() {
            Err(Error::DatabaseUrlIsInvalid(
                path.to_str().unwrap().to_string(),
            ))
        } else {
            let official = vec![];
            let mut connection_limit = 1;

            if path_parts.len() > 1 {
                let (_, unsupported): ConnectionParams = path_parts
                    .last()
                    .unwrap()
                    .split("&")
                    .map(|kv| {
                        let splitted: Vec<&str> = kv.split("=").collect();
                        (String::from(splitted[0]), String::from(splitted[1]))
                    })
                    .collect::<Vec<(String, String)>>()
                    .into_iter()
                    .partition(|(k, _)| official.contains(&k.as_str()));

                for (k, v) in unsupported.into_iter() {
                    match k.as_ref() {
                        "connection_limit" => {
                            let as_int: u32 =
                                v.parse().map_err(|_| Error::InvalidConnectionArguments)?;
                            connection_limit = as_int;
                        }
                        _ => trace!("Discarding connection string param: {}", k),
                    };
                }
            }

            Ok(Self {
                connection_limit,
                file_path: path,
            })
        }
    }
}

impl TryFrom<&str> for Sqlite {
    type Error = Error;

    fn try_from(path: &str) -> crate::Result<Self> {
        let params = SqliteParams::try_from(path)?;
        let client = rusqlite::Connection::open_in_memory()?;
        let file_path = params.file_path;

        Ok(Sqlite { client, file_path })
    }
}

impl Sqlite {
    pub fn new<P>(file_path: P) -> crate::Result<Sqlite>
    where
        P: Into<PathBuf>,
    {
        let client = rusqlite::Connection::open_in_memory()?;

        Ok(Sqlite {
            client,
            file_path: file_path.into(),
        })
    }

    pub fn attach_database(&mut self, db_name: &str) -> crate::Result<()> {
        let mut stmt = self.client.prepare("PRAGMA database_list")?;

        let databases: HashSet<String> = stmt
            .query_map(NO_PARAMS, |row| {
                let name: String = row.get(1)?;

                Ok(name)
            })?
            .map(|res| res.unwrap())
            .collect();

        if !databases.contains(db_name) {
            rusqlite::Connection::execute(
                &self.client,
                "ATTACH DATABASE ? AS ?",
                &[self.file_path.to_str().unwrap(), db_name],
            )?;
        }

        rusqlite::Connection::execute(&self.client, "PRAGMA foreign_keys = ON", NO_PARAMS)?;

        Ok(())
    }
}

impl Queryable for Sqlite {
    fn execute(&mut self, q: Query) -> crate::Result<Option<Id>> {
        let (sql, params) = visitor::Sqlite::build(q);

        self.execute_raw(&sql, &params)?;

        Ok(Some(Id::Int(self.client.last_insert_rowid() as usize)))
    }

    fn query(&mut self, q: Query) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Sqlite::build(q);

        self.query_raw(&sql, &params)
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<ResultSet> {
        let mut stmt = self.client.prepare_cached(sql)?;
        let mut rows = stmt.query(params)?;

        let mut result = ResultSet::new(rows.to_column_names(), Vec::new());

        while let Some(row) = rows.next()? {
            result.rows.push(row.to_result_row()?);
        }

        Ok(result)
    }

    fn execute_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<u64> {
        let mut stmt = self.client.prepare_cached(sql)?;
        let changes = stmt.execute(params)?;

        Ok(u64::try_from(changes).unwrap())
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("PRAGMA foreign_keys = OFF", &[])?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("PRAGMA foreign_keys = ON", &[])?;
        Ok(())
    }

    fn start_transaction<'b>(&'b mut self) -> crate::Result<Transaction<'b>> {
        Ok(Transaction::new(self)?)
    }

    fn raw_cmd(&mut self, cmd: &str) -> crate::Result<()> {
        self.client.execute_batch(cmd)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::Queryable;

    #[test]
    fn should_provide_a_database_connection() {
        let mut connection = Sqlite::new(String::from("db/test.db")).unwrap();
        let res = connection
            .query_raw("SELECT * FROM sqlite_master", &[])
            .unwrap();

        assert!(res.is_empty());
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let mut connection = Sqlite::new(String::from("db/test.db")).unwrap();
        let mut tx = connection.start_transaction().unwrap();
        let res = tx.query_raw("SELECT * FROM sqlite_master", &[]).unwrap();

        assert!(res.is_empty());
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
        let mut connection = Sqlite::try_from("file:db/test.db").unwrap();

        connection.query_raw(TABLE_DEF, &[]).unwrap();
        connection.query_raw(CREATE_USER, &[]).unwrap();

        let rows = connection.query_raw("SELECT * FROM USER", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["ID"].as_i64(), Some(1));
        assert_eq!(row["NAME"].as_str(), Some("Joe"));
        assert_eq!(row["AGE"].as_i64(), Some(27));
        assert_eq!(row["SALARY"].as_f64(), Some(20000.0));
    }
}
