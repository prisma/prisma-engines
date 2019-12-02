use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithUnionAll, FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use quaint::pooled::Quaint;

pub struct Mysql {
    pool: Quaint,
}

#[async_trait]
impl FromSource for Mysql {
    async fn from_source(source: &dyn Source) -> crate::Result<Self> {
        Ok(Mysql {
            pool: Quaint::new(&source.url().value).await?,
        })
    }
}

impl Connector for Mysql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithUnionAll>::new(conn);

            Ok(Box::new(conn) as Box<dyn Connection>)
        })
    }
}
