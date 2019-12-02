use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError};
use connector_interface::{Connection, Connector, error::ConnectorError, IO};
use datamodel::Source;
use quaint::pooled::Quaint;
use async_trait::async_trait;

pub struct PostgreSql {
    pool: Quaint,
    connection_info: quaint::prelude::ConnectionInfo,
}

impl PostgreSql {
    async fn catch<O>(&self, fut: impl std::future::Future<Output = Result<O, crate::SqlError>>) -> Result<O, ConnectorError> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(err) => Err(err.into_connector_error(&self.connection_info)),
        }
    }
}

#[async_trait]
impl FromSource for PostgreSql {
    async fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let pool = Quaint::new(&source.url().value).await?;
        let connection_info = pool.connection_info().to_owned();
        Ok(PostgreSql {
            pool,
            connection_info,
        })
    }
}

impl Connector for PostgreSql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(self.catch(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithRowNumber>::new(conn, &self.connection_info);

            Ok(Box::new(conn) as Box<dyn Connection>)
        }))
    }
}
