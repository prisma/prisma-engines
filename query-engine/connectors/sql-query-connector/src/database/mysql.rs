use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithUnionAll, FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{Connection, Connector, error::ConnectorError, IO};
use datamodel::Source;
use quaint::pooled::Quaint;

pub struct Mysql {
    pool: Quaint,
    connection_info: quaint::prelude::ConnectionInfo,
}

impl Mysql {
    async fn catch<O>(&self, fut: impl std::future::Future<Output = Result<O, crate::SqlError>>) -> Result<O, ConnectorError> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(err) => Err(err.into_connector_error(&self.connection_info)),
        }
    }
}

#[async_trait]
impl FromSource for Mysql {
    async fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let pool = Quaint::new(&source.url().value)?;
        let connection_info = pool.connection_info().to_owned();
        Ok(Mysql {
            pool: Quaint::new(&source.url().value).await?,
            connection_info,
        })
    }
}

impl Connector for Mysql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(self.catch(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithUnionAll>::new(conn, &self.connection_info);

            Ok(Box::new(conn) as Box<dyn Connection>)
        }))
    }
}
