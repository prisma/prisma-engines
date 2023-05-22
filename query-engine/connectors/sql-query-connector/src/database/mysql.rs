use super::connection::SqlConnection;
use crate::{query_ext::QueryExt, FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use quaint::{
    pooled::{PooledConnection, Quaint},
    prelude::{ConnectionInfo, Queryable, TransactionCapable},
};
use std::time::Duration;

pub struct NodeJSPool;

pub enum MysqlPool {
    Rust(Quaint),
    NodeJS(NodeJSPool),
}

impl MysqlPool {
    /// Reserve a connection from the pool
    /// TODO: the resulting dyn trait should also be `TransactionCapable` with an automatic implementation.
    /// Note: `TransactionCapable` requires `Self: Sized`, which is not possible with a trait object.
    /// We could implement `TransactionCapable` for `Box<dyn Connection + Send + Sync + 'static>`,
    /// but that would yield the error:
    /// "only traits defined in the current crate can be implemented for types defined outside of the crate.
    /// define and implement a trait or new type instead".
    /// I've also already tried adding a generic constraint to `TransactionCapable`, but that didn't help solving
    /// the problems above.
    pub async fn check_out(&self) -> crate::Result<Box<dyn QueryExt + Send + Sync + 'static>> {
        match self {
            MysqlPool::Rust(pool) => {
                let conn: PooledConnection = pool.check_out().await.map_err(SqlError::from)?;
                let obj = Box::new(conn);
                Ok(obj)
            }
            MysqlPool::NodeJS(_) => unimplemented!("NodeJS connection pool"),
        }
    }
}

pub struct Mysql {
    pool: MysqlPool,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
}

impl Mysql {
    /// Get MySQL's preview features.
    pub fn features(&self) -> psl::PreviewFeatures {
        self.features
    }
}

#[async_trait]
impl FromSource for Mysql {
    async fn from_source(
        _source: &psl::Datasource,
        url: &str,
        features: psl::PreviewFeatures,
    ) -> connector_interface::Result<Mysql> {
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

        Ok(Mysql {
            pool: MysqlPool::Rust(pool),
            connection_info,
            features: features.to_owned(),
        })
    }
}

// Note: implementing something like
// `trait NewTrait: Connection + TransactionCapable<(dyn Connection + std::marker::Send + Sync + 'static)> {}`
// and making `get_connection` return `Result<Box<dyn NewTrait>>`, would result in the error:
// `the trait `NewTrait` cannot be made into an object`

#[async_trait]
impl Connector for Mysql {
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + Send + Sync + 'static>> {
        super::catch(self.connection_info.clone(), async move {
            let conn = self.pool.check_out().await?;
            let conn = SqlConnection::new(conn, &self.connection_info, self.features);

            // TODO: this line fails due to reasons explained in the other comments in this file.
            Ok(Box::new(conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> &'static str {
        "mysql"
    }

    fn should_retry_on_transient_error(&self) -> bool {
        false
    }
}
