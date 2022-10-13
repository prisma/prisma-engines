use super::connection::SqlConnection;
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use psl::{Datasource, PreviewFeature};
use quaint::{pooled::Quaint, prelude::ConnectionInfo};
use std::time::Duration;

pub struct PostgreSql {
    pool: Quaint,
    connection_info: ConnectionInfo,
    features: Vec<PreviewFeature>,
}

impl PostgreSql {
    /// Get PostgreSQL's preview features.
    pub fn features(&self) -> &[PreviewFeature] {
        self.features.as_ref()
    }
}

#[async_trait]
impl FromSource for PostgreSql {
    async fn from_source(
        _source: &Datasource,
        url: &str,
        features: &[PreviewFeature],
    ) -> connector_interface::Result<Self> {
        let database_str = url;

        let connection_info = ConnectionInfo::from_url(database_str).map_err(|err| {
            ConnectorError::from_kind(ErrorKind::InvalidDatabaseUrl {
                details: err.to_string(),
                url: database_str.to_string(),
            })
        })?;

        let mut builder = Quaint::builder(url)
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        builder.health_check_interval(Duration::from_secs(15));
        builder.test_on_check_out(true);

        let pool = builder.build();
        let connection_info = pool.connection_info().to_owned();
        Ok(PostgreSql {
            pool,
            connection_info,
            features: features.to_owned(),
        })
    }
}

#[async_trait]
impl Connector for PostgreSql {
    async fn get_connection<'a>(&'a self) -> connector_interface::Result<Box<dyn Connection + Send + Sync + 'static>> {
        super::catch(self.connection_info.clone(), async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, &self.connection_info, self.features.clone());
            Ok(Box::new(conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> String {
        "postgres".to_owned()
    }
}
