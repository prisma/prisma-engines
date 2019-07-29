use super::PrismaConnectionManager;
use crate::{
    connector::{PostgreSql, Queryable, PostgresParams, DEFAULT_SCHEMA},
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

impl TryFrom<Config> for PrismaConnectionManager<PostgresManager> {
    type Error = Error;

    fn try_from(opts: postgres::Config) -> crate::Result<Self> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku

        let tls = MakeTlsConnector::new(tls_builder.build()?);

        Ok(Self {
            inner: PostgresConnectionManager::new(opts, tls),
            file_path: None,
            schema: None,
        })
    }
}

impl TryFrom<Url> for PrismaConnectionManager<PostgresManager> {
    type Error = Error;

    fn try_from(url: Url) -> crate::Result<Self> {
        let params = PostgresParams::try_from(url)?;
        Self::try_from(params.config)
    }
}

impl ManageConnection for PrismaConnectionManager<PostgresManager> {
    type Connection = PostgreSql;
    type Error = Compat<Error>;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        match self.inner.connect() {
            Ok(mut client) => {
                let schema = self.schema.as_ref().map(|s| s.as_str()).unwrap_or(DEFAULT_SCHEMA);

                match client.execute(format!("SET search_path = {}", schema).as_str(), &[]) {
                    Ok(_) => Ok(PostgreSql::from(client)),
                    Err(e) => Err(Error::from(e).compat()),
                }
            },
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
