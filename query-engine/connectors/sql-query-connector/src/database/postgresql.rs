use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    error::{ConnectorError, ErrorKind},
    Connection, Connector, IO,
};
use datamodel::Source;
use quaint::{pooled::Quaint, prelude::ConnectionInfo};

pub struct PostgreSql {
    pool: Quaint,
    connection_info: ConnectionInfo,
}

#[async_trait]
impl FromSource for PostgreSql {
    async fn from_source(source: &dyn Source) -> connector_interface::Result<Self> {
        let connection_info = ConnectionInfo::from_url(&source.url().value)
            .map_err(|err| ConnectorError::from_kind(ErrorKind::ConnectionError(err.into())))?;

        let pool = Quaint::new(&source.url().value)
            .await
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;
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
