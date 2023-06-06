use super::connection::SqlConnection;
use super::nodejs::{RuntimeConnection, RuntimePool};
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use quaint::{
    pooled::{PooledConnection, Quaint},
    prelude::ConnectionInfo,
};
use std::time::Duration;

impl RuntimePool {
    /// Reserve a connection from the pool
    pub async fn check_out(&self) -> crate::Result<RuntimeConnection> {
        match self {
            Self::Rust(pool) => {
                let conn: PooledConnection = pool.check_out().await.map_err(SqlError::from)?;
                Ok(RuntimeConnection::Rust(conn))
            }
            #[cfg(feature = "nodejs-drivers")]
            Self::NodeJS(queryable) => Ok(RuntimeConnection::NodeJS(queryable.clone())),
        }
    }
}

pub struct Mysql {
    pool: RuntimePool,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
}

impl Mysql {
    /// Get MySQL's preview features.
    pub fn features(&self) -> psl::PreviewFeatures {
        self.features
    }
}

fn get_connection_info(url: &str) -> connector::Result<ConnectionInfo> {
    let database_str = url;

    let connection_info = ConnectionInfo::from_url(database_str).map_err(|err| {
        ConnectorError::from_kind(ErrorKind::InvalidDatabaseUrl {
            details: err.to_string(),
            url: database_str.to_string(),
        })
    })?;

    Ok(connection_info)
}

#[async_trait]
impl FromSource for Mysql {
    async fn from_source(
        source: &psl::Datasource,
        url: &str,
        features: psl::PreviewFeatures,
    ) -> connector_interface::Result<Mysql> {
        if source.provider == "@prisma/mysql" {
            #[cfg(feature = "nodejs-drivers")]
            {
                let queryable = nodejs_drivers::installed_driver().unwrap().clone();
                let connection_info = get_connection_info(url)?;
                let pool = RuntimePool::NodeJS(queryable);

                return Ok(Mysql {
                    pool,
                    connection_info,
                    features: features.to_owned(),
                });
            }

            #[cfg(not(feature = "nodejs-drivers"))]
            {
                return Err(ConnectorError::from_kind(ErrorKind::UnsupportedConnector(
                    "The @prisma/mysql connector requires the `nodejs-drivers` feature to be enabled.".into(),
                )));
            }
        }

        let connection_info = get_connection_info(url)?;

        let mut builder = Quaint::builder(url)
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        builder.health_check_interval(Duration::from_secs(15));
        builder.test_on_check_out(true);

        let pool = builder.build();
        let connection_info = pool.connection_info().to_owned();

        Ok(Mysql {
            pool: RuntimePool::Rust(pool),
            connection_info,
            features: features.to_owned(),
        })
    }
}

#[async_trait]
impl Connector for Mysql {
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + Send + Sync + 'static>> {
        super::catch(self.connection_info.clone(), async move {
            let runtime_conn = self.pool.check_out().await?;

            // Note: `runtime_conn` must be `Sized`, as that's required by `TransactionCapable`
            let sql_conn = SqlConnection::new(runtime_conn, &self.connection_info, self.features);

            Ok(Box::new(sql_conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> &'static str {
        if self.pool.is_nodejs() {
            "@prisma/mysql"
        } else {
            "mysql"
        }
    }

    fn should_retry_on_transient_error(&self) -> bool {
        false
    }
}
