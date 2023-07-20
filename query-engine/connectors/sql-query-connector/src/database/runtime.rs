use crate::SqlError;
use async_trait::async_trait;
use quaint::{
    connector::IsolationLevel,
    pooled::{PooledConnection, Quaint},
    prelude::{Query, Queryable, TransactionCapable},
    Value,
};

#[cfg(feature = "js-connectors")]
type QueryableRef = std::sync::Arc<dyn Queryable>;

pub enum RuntimePool {
    Rust(Quaint),

    #[cfg(feature = "js-connectors")]
    Js(QueryableRef),
}

impl RuntimePool {
    pub fn is_nodejs(&self) -> bool {
        match self {
            Self::Rust(_) => false,

            #[cfg(feature = "js-connectors")]
            Self::Js(_) => true,
        }
    }

    /// Reserve a connection from the pool
    pub async fn check_out(&self) -> crate::Result<RuntimeConnection> {
        match self {
            Self::Rust(pool) => {
                let conn: PooledConnection = pool.check_out().await.map_err(SqlError::from)?;
                Ok(RuntimeConnection::Rust(conn))
            }
            #[cfg(feature = "js-connectors")]
            Self::Js(queryable) => Ok(RuntimeConnection::Js(queryable.clone())),
        }
    }
}

pub enum RuntimeConnection {
    Rust(PooledConnection),

    #[cfg(feature = "js-connectors")]
    Js(QueryableRef),
}

#[async_trait]
impl Queryable for RuntimeConnection {
    async fn query(&self, q: Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        match self {
            Self::Rust(conn) => conn.query(q).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.query(q).await,
        }
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        match self {
            Self::Rust(conn) => conn.query_raw(sql, params).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.query_raw(sql, params).await,
        }
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        match self {
            Self::Rust(conn) => conn.query_raw_typed(sql, params).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.query_raw_typed(sql, params).await,
        }
    }

    async fn execute(&self, q: Query<'_>) -> quaint::Result<u64> {
        match self {
            Self::Rust(conn) => conn.execute(q).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.execute(q).await,
        }
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        match self {
            Self::Rust(conn) => conn.execute_raw(sql, params).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.execute_raw(sql, params).await,
        }
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        match self {
            Self::Rust(conn) => conn.execute_raw_typed(sql, params).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.execute_raw_typed(sql, params).await,
        }
    }

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        match self {
            Self::Rust(conn) => conn.raw_cmd(cmd).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.raw_cmd(cmd).await,
        }
    }

    async fn version(&self) -> quaint::Result<Option<String>> {
        match self {
            Self::Rust(conn) => conn.version().await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.version().await,
        }
    }

    fn is_healthy(&self) -> bool {
        match self {
            Self::Rust(conn) => conn.is_healthy(),

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.is_healthy(),
        }
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        match self {
            Self::Rust(conn) => conn.set_tx_isolation_level(isolation_level).await,

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.set_tx_isolation_level(isolation_level).await,
        }
    }

    /// Signals if the isolation level SET needs to happen before or after the tx BEGIN.
    fn requires_isolation_first(&self) -> bool {
        match self {
            Self::Rust(conn) => conn.requires_isolation_first(),

            #[cfg(feature = "js-connectors")]
            Self::Js(conn) => conn.requires_isolation_first(),
        }
    }
}

impl TransactionCapable for RuntimeConnection {}
