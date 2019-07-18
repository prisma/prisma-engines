use super::PrismaConnectionManager;
use crate::{
    connector::{Queryable, Sqlite},
    error::Error,
};
use failure::{Compat, Fail};
use r2d2::ManageConnection;
use std::path::PathBuf;

pub use r2d2_sqlite::SqliteConnectionManager;

impl PrismaConnectionManager<SqliteConnectionManager> {
    pub fn new(url: &str) -> crate::Result<Self> {
        let normalized = url.trim_start_matches("file:");
        let path = PathBuf::from(&normalized);

        if path.is_dir() {
            Err(Error::DatabaseUrlIsInvalid(url.to_string()))
        } else {
            Ok(Self {
                inner: SqliteConnectionManager::memory(),
                file_path: Some(path),
            })
        }
    }
}

impl ManageConnection for PrismaConnectionManager<SqliteConnectionManager> {
    type Connection = Sqlite;
    type Error = Compat<Error>;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        match self.inner.connect() {
            Ok(client) => {
                let sqlite = Sqlite {
                    client,
                    file_path: self.file_path.clone().unwrap(),
                };

                Ok(sqlite)
            }
            Err(e) => Err(Error::from(e).compat()),
        }
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        match conn.query_raw("", &[]) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::from(e).compat()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}
