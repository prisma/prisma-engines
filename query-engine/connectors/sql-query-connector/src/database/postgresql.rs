use super::connection::SqlConnection;
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    error::{ConnectorError, ErrorKind},
    Connection, Connector, ConnectorCapabilities, ConnectorCapability,
};
use datamodel::Datasource;
use quaint::{pooled::Quaint, prelude::ConnectionInfo};
use std::time::Duration;

pub struct PostgreSql {
    pool: Quaint,
    connection_info: ConnectionInfo,
}

#[async_trait]
impl FromSource for PostgreSql {
    async fn from_source(source: &Datasource) -> connector_interface::Result<Self> {
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
        Ok(PostgreSql { pool, connection_info })
    }
}

#[async_trait]
impl Connector for PostgreSql {
    async fn get_connection<'a>(&'a self) -> connector_interface::Result<Box<dyn Connection + 'static>> {
        super::catch(&self.connection_info, async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, &self.connection_info);
            Ok(Box::new(conn) as Box<dyn Connection>)
        })
        .await
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        let capabilities = ConnectorCapabilities::default();
        capabilities.add(ConnectorCapability::InsensitiveFilters)
    }

    fn name(&self) -> String {
        "postgres".to_owned()
    }
}
