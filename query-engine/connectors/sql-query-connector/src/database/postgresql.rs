use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use quaint::pooled::Quaint;

pub struct PostgreSql {
    pool: Quaint,
    connection_info: quaint::prelude::ConnectionInfo,
}

#[async_trait]
impl FromSource for PostgreSql {
    async fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let pool = Quaint::new(&source.url().value).await?;
        let connection_info = pool.connection_info().to_owned();
        Ok(PostgreSql { pool, connection_info })
    }
}

impl Connector for PostgreSql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(super::catch(&self.connection_info, async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithRowNumber>::new(conn, &self.connection_info);

            Ok(Box::new(conn) as Box<dyn Connection>)
        }))
    }
}
