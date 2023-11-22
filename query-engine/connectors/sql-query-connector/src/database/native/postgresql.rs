use crate::database::{catch, connection::SqlConnection};
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use psl::builtin_connectors::COCKROACH;
use quaint::{connector::PostgresFlavour, pooled::Quaint, prelude::ConnectionInfo};
use std::time::Duration;

pub struct PostgreSql {
    pool: Quaint,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
}

impl PostgreSql {
    /// Get PostgreSQL's preview features.
    pub fn features(&self) -> psl::PreviewFeatures {
        self.features
    }
}

#[async_trait]
impl FromSource for PostgreSql {
    async fn from_source(
        source: &psl::Datasource,
        url: &str,
        features: psl::PreviewFeatures,
    ) -> connector_interface::Result<Self> {
        let database_str = url;

        // This connection info is only used for error rendering. It does not matter that the flavour is not set.
        let err_conn_info = ConnectionInfo::from_url(url).map_err(|err| {
            ConnectorError::from_kind(ErrorKind::InvalidDatabaseUrl {
                details: err.to_string(),
                url: database_str.to_string(),
            })
        })?;

        let mut builder = Quaint::builder(url)
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&err_conn_info))?;

        let flavour = if COCKROACH.is_provider(source.active_provider) {
            PostgresFlavour::Cockroach
        } else {
            PostgresFlavour::Postgres
        };

        // The postgres flavour is set in order to avoid a network roundtrip when connecting to the database.
        builder.set_postgres_flavour(flavour);
        builder.health_check_interval(Duration::from_secs(15));
        builder.test_on_check_out(true);

        let pool = builder.build();
        let connection_info = pool.connection_info().to_owned();
        Ok(PostgreSql {
            pool,
            connection_info,
            features,
        })
    }
}

#[async_trait]
impl Connector for PostgreSql {
    async fn get_connection<'a>(&'a self) -> connector_interface::Result<Box<dyn Connection + Send + Sync + 'static>> {
        catch(self.connection_info.clone(), async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, &self.connection_info, self.features);
            Ok(Box::new(conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> &'static str {
        "postgres"
    }

    fn should_retry_on_transient_error(&self) -> bool {
        false
    }
}
