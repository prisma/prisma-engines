use super::connection::SqlConnection;
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    error::{ConnectorError, ErrorKind},
    Connection, Connector, IO,
};
use datamodel::Source;
use quaint::{pooled::Quaint, prelude::ConnectionInfo};
use std::time::Duration;

pub struct Mysql {
    pool: Quaint,
    connection_info: ConnectionInfo,
}

#[async_trait]
impl FromSource for Mysql {
    async fn from_source(source: &dyn Source) -> connector_interface::Result<Self> {
        let connection_info = ConnectionInfo::from_url(&source.url().value)
            .map_err(|err| ConnectorError::from_kind(ErrorKind::ConnectionError(err.into())))?;

        let mut builder = Quaint::builder(&source.url().value)
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        builder.max_idle_lifetime(Duration::from_secs(300));
        builder.health_check_interval(Duration::from_secs(15));
        builder.test_on_check_out(true);

        let pool = builder.build();
        let connection_info = pool.connection_info().to_owned();

        Ok(Mysql { pool, connection_info })
    }
}

impl Connector for Mysql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(super::catch(&self.connection_info, async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, &self.connection_info);

            Ok(Box::new(conn) as Box<dyn Connection>)
        }))
    }
}
