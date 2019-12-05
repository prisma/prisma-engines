use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithUnionAll, FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use quaint::pooled::Quaint;

pub struct Mysql {
    pool: Quaint,
    connection_info: quaint::prelude::ConnectionInfo,
}

#[async_trait]
impl FromSource for Mysql {
    async fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let pool = Quaint::new(&source.url().value).await?;
        let connection_info = pool.connection_info().to_owned();
        Ok(Mysql {
            pool,
            connection_info,
        })
    }
}

impl Connector for Mysql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(super::catch(&self.connection_info, async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithUnionAll>::new(conn, &self.connection_info);

            Ok(Box::new(conn) as Box<dyn Connection>)
        }))
    }
}
