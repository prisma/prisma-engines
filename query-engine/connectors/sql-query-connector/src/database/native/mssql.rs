use crate::database::{catch, connection::SqlConnection};
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    self as connector, Connection, Connector,
    error::{ConnectorError, ErrorKind},
};
use psl::{Datasource, PreviewFeatures};
use quaint::{pooled::Quaint, prelude::ConnectionInfo};
use std::time::Duration;

pub struct Mssql {
    pool: Quaint,
    connection_info: ConnectionInfo,
    features: PreviewFeatures,
}

impl Mssql {
    /// Get MSSQL's preview features.
    pub fn features(&self) -> PreviewFeatures {
        self.features
    }
}

#[async_trait]
impl FromSource for Mssql {
    async fn from_source(
        _source: &Datasource,
        url: &str,
        features: PreviewFeatures,
        _tracing_enabled: bool,
    ) -> connector_interface::Result<Self> {
        let database_str = url;

        let connection_info = ConnectionInfo::from_url(database_str).map_err(|err| {
            ConnectorError::from_kind(ErrorKind::InvalidDatabaseUrl {
                details: err.to_string(),
                url: database_str.to_string(),
            })
        })?;

        let mut builder = Quaint::builder(database_str)
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(connection_info.as_native()))?;

        builder.health_check_interval(Duration::from_secs(15));
        builder.test_on_check_out(true);

        let pool = builder.build();
        let connection_info = pool.connection_info().to_owned();

        Ok(Self {
            pool,
            connection_info,
            features: features.to_owned(),
        })
    }
}

#[async_trait]
impl Connector for Mssql {
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + Send + Sync + 'static>> {
        catch(&self.connection_info, async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, self.connection_info.clone(), self.features);

            Ok(Box::new(conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> &'static str {
        "mssql"
    }

    fn should_retry_on_transient_error(&self) -> bool {
        false
    }
}
