use super::PrismaConnectionManager;
use crate::{
    connector::{metrics, Queryable, Sqlite, SqliteParams},
    error::Error,
};
use failure::{Compat, Fail};
use r2d2::ManageConnection;
use std::convert::TryFrom;

pub use r2d2_sqlite::SqliteConnectionManager;

impl TryFrom<SqliteParams> for r2d2::Pool<PrismaConnectionManager<SqliteConnectionManager>> {
    type Error = Error;

    fn try_from(params: SqliteParams) -> crate::Result<Self> {
        let manager =
            PrismaConnectionManager::sqlite(params.schema, params.file_path.to_str().unwrap())?;

        let pool = r2d2::Pool::builder()
            .max_size(params.connection_limit)
            .test_on_check_out(false)
            .build(manager)?;

        Ok(pool)
    }
}

impl PrismaConnectionManager<SqliteConnectionManager> {
    pub fn sqlite(db_name: Option<String>, path: &str) -> crate::Result<Self> {
        let params = SqliteParams::try_from(path)?;

        Ok(Self {
            inner: SqliteConnectionManager::memory(),
            file_path: Some(params.file_path),
            schema: db_name,
        })
    }
}

impl ManageConnection for PrismaConnectionManager<SqliteConnectionManager> {
    type Connection = Sqlite;
    type Error = Compat<Error>;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        match metrics::connect("pool.sqlite", || self.inner.connect()) {
            Ok(client) => {
                let mut sqlite = Sqlite {
                    client,
                    file_path: self.file_path.clone().unwrap(),
                };

                if let Some(ref schema) = self.schema {
                    sqlite
                        .attach_database(schema)
                        .map_err(|err| Error::from(err).compat())?;
                }

                Ok(sqlite)
            }
            Err(e) => Err(Error::from(e).compat()),
        }
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        match conn.query_raw("SELECT 1", &[]) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.compat()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_connection_limit() {
        let conn_string = format!("file:db/test.db",);
        let params = SqliteParams::try_from(conn_string.as_str()).unwrap();
        let pool = r2d2::Pool::try_from(params).unwrap();

        assert_eq!(num_cpus::get() * 2 + 4, pool.max_size() as usize);
    }

    #[test]
    fn test_custom_connection_limit() {
        let conn_string = format!("file:db/test.db?connection_limit=10",);

        let params = SqliteParams::try_from(conn_string.as_str()).unwrap();
        let pool = r2d2::Pool::try_from(params).unwrap();

        assert_eq!(10, pool.max_size());
    }
}
