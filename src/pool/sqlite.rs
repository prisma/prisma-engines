use super::PrismaConnectionManager;
use crate::{
    connector::{Queryable, Sqlite, SqliteParams},
    error::Error,
};
use failure::{Compat, Fail};
use r2d2::ManageConnection;
use std::{convert::TryFrom, path::PathBuf};
use url::Url;

pub use r2d2_sqlite::SqliteConnectionManager;

impl TryFrom<SqliteParams> for r2d2::Pool<PrismaConnectionManager<SqliteConnectionManager>> {
    type Error = Error;

    fn try_from(params: SqliteParams) -> crate::Result<Self> {
        let manager = PrismaConnectionManager::sqlite(params.file_path)?;

        let pool = r2d2::Pool::builder()
            .max_size(params.connection_limit)
            .build(manager)?;

        Ok(pool)
    }
}

impl PrismaConnectionManager<SqliteConnectionManager> {
    pub fn sqlite<P>(pbuf: P) -> crate::Result<Self>
    where
        P: Into<PathBuf>,
    {
        let path = pbuf.into();

        if path.is_dir() {
            Err(Error::DatabaseUrlIsInvalid(String::from(path.to_str().unwrap())))
        } else {
            Ok(Self {
                inner: SqliteConnectionManager::memory(),
                file_path: Some(path),
                schema: None,
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
        match conn.query_raw("SELECT 1", &[]) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::from(e).compat()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use url::Url;

    #[test]
    fn test_default_connection_limit() {
        let conn_string = format!(
            "file:///home/naukio/file.db",
        );

        let url = Url::parse(&conn_string).unwrap();
        let params = SqliteParams::try_from(url).unwrap();
        let pool = r2d2::Pool::try_from(params).unwrap();

        assert_eq!(1, pool.max_size());
    }

    #[test]
    fn test_custom_connection_limit() {
        let conn_string = format!(
            "file:///home/naukio/file.db?connection_limit=10",
        );

        let url = Url::parse(&conn_string).unwrap();
        let params = SqliteParams::try_from(url).unwrap();
        let pool = r2d2::Pool::try_from(params).unwrap();

        assert_eq!(10, pool.max_size());
    }
}
