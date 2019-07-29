use super::PrismaConnectionManager;
use crate::{
    connector::{PostgreSql, PostgresParams, Queryable, DEFAULT_SCHEMA},
    error::Error,
};
use failure::{Compat, Fail};
use native_tls::TlsConnector;
use r2d2::ManageConnection;
use std::convert::TryFrom;
use tokio_postgres_native_tls::MakeTlsConnector;
use url::Url;

pub use postgres::Config;
pub use r2d2_postgres::PostgresConnectionManager;

pub type PostgresManager = PostgresConnectionManager<MakeTlsConnector>;

impl TryFrom<Url> for PrismaConnectionManager<PostgresManager> {
    type Error = Error;

    fn try_from(url: Url) -> crate::Result<Self> {
        let params = PostgresParams::try_from(url)?;
        Self::postgres(params.config, Some(params.schema))
    }
}

impl PrismaConnectionManager<PostgresManager> {
    pub fn postgres(opts: postgres::Config, schema: Option<String>) -> crate::Result<Self> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku

        let tls = MakeTlsConnector::new(tls_builder.build()?);

        Ok(Self {
            inner: PostgresConnectionManager::new(opts, tls),
            file_path: None,
            schema,
        })
    }
}

impl ManageConnection for PrismaConnectionManager<PostgresManager> {
    type Connection = PostgreSql;
    type Error = Compat<Error>;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        match self.inner.connect() {
            Ok(mut client) => {
                let schema = self
                    .schema
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_SCHEMA);

                match client.execute(format!("SET search_path = {}", schema).as_str(), &[]) {
                    Ok(_) => Ok(PostgreSql::from(client)),
                    Err(e) => Err(Error::from(e).compat()),
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use url::Url;

    #[test]
    fn test_default_connection_limit() {
        let conn_string = format!(
            "postgresql://{}:{}@{}:{}/{}",
            env::var("TEST_PG_USER").unwrap(),
            env::var("TEST_PG_PASSWORD").unwrap(),
            env::var("TEST_PG_HOST").unwrap(),
            env::var("TEST_PG_PORT").unwrap(),
            env::var("TEST_PG_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let params = PostgresParams::try_from(url).unwrap();

        let manager = PrismaConnectionManager::postgres(params.config, None).unwrap();

        let pool = r2d2::Pool::builder()
            .max_size(params.connection_limit)
            .build(manager)
            .unwrap();

        assert_eq!(1, pool.max_size());
    }

    #[test]
    fn test_custom_connection_limit() {
        let conn_string = format!(
            "postgresql://{}:{}@{}:{}/{}?connection_limit=10",
            env::var("TEST_PG_USER").unwrap(),
            env::var("TEST_PG_PASSWORD").unwrap(),
            env::var("TEST_PG_HOST").unwrap(),
            env::var("TEST_PG_PORT").unwrap(),
            env::var("TEST_PG_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let params = PostgresParams::try_from(url).unwrap();

        let manager = PrismaConnectionManager::postgres(params.config, None).unwrap();

        let pool = r2d2::Pool::builder()
            .max_size(params.connection_limit)
            .build(manager)
            .unwrap();

        assert_eq!(10, pool.max_size());
    }

    #[test]
    fn test_custom_search_path() {
        let conn_string = format!(
            "postgresql://{}:{}@{}:{}/{}?schema=musti",
            env::var("TEST_PG_USER").unwrap(),
            env::var("TEST_PG_PASSWORD").unwrap(),
            env::var("TEST_PG_HOST").unwrap(),
            env::var("TEST_PG_PORT").unwrap(),
            env::var("TEST_PG_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let manager = PrismaConnectionManager::try_from(url).unwrap();

        let pool = r2d2::Pool::builder().build(manager).unwrap();

        let mut conn = pool.get().unwrap();
        let result_set = conn.query_raw("SHOW search_path", &[]).unwrap();
        let row = result_set.first().unwrap();

        assert_eq!(Some("musti"), row[0].as_str());
    }
}
