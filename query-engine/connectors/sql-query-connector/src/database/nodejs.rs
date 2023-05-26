use async_trait::async_trait;
use nodejs_drivers::{pool::NodeJSPool, queryable::NodeJSQueryable};
use quaint::{
    connector::IsolationLevel,
    pooled::{PooledConnection, Quaint},
    prelude::{Query, Queryable, TransactionCapable},
    Value,
};

pub enum RuntimePool {
    Rust(Quaint),
    NodeJS(NodeJSPool),
}

impl RuntimePool {
    pub fn is_nodejs(&self) -> bool {
        match self {
            Self::Rust(_) => false,
            Self::NodeJS(_) => true,
        }
    }
}

pub enum RuntimeConnection {
    Rust(PooledConnection),
    NodeJS(NodeJSQueryable),
}

#[async_trait]
impl Queryable for RuntimeConnection {
    async fn query(&self, q: Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        match self {
            Self::Rust(conn) => conn.query(q).await,
            Self::NodeJS(conn) => conn.query(q).await,
        }
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        match self {
            Self::Rust(conn) => conn.query_raw(sql, params).await,
            Self::NodeJS(conn) => conn.query_raw(sql, params).await,
        }
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        match self {
            Self::Rust(conn) => conn.query_raw_typed(sql, params).await,
            Self::NodeJS(conn) => conn.query_raw_typed(sql, params).await,
        }
    }

    async fn execute(&self, q: Query<'_>) -> quaint::Result<u64> {
        match self {
            Self::Rust(conn) => conn.execute(q).await,
            Self::NodeJS(conn) => conn.execute(q).await,
        }
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        match self {
            Self::Rust(conn) => conn.execute_raw(sql, params).await,
            Self::NodeJS(conn) => conn.execute_raw(sql, params).await,
        }
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        match self {
            Self::Rust(conn) => conn.execute_raw_typed(sql, params).await,
            Self::NodeJS(conn) => conn.execute_raw_typed(sql, params).await,
        }
    }

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        match self {
            Self::Rust(conn) => conn.raw_cmd(cmd).await,
            Self::NodeJS(conn) => conn.raw_cmd(cmd).await,
        }
    }

    async fn version(&self) -> quaint::Result<Option<String>> {
        match self {
            Self::Rust(conn) => conn.version().await,
            Self::NodeJS(conn) => conn.version().await,
        }
    }

    fn is_healthy(&self) -> bool {
        match self {
            Self::Rust(conn) => conn.is_healthy(),
            Self::NodeJS(conn) => conn.is_healthy(),
        }
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        match self {
            Self::Rust(conn) => conn.set_tx_isolation_level(isolation_level).await,
            Self::NodeJS(conn) => conn.set_tx_isolation_level(isolation_level).await,
        }
    }

    /// Signals if the isolation level SET needs to happen before or after the tx BEGIN.
    fn requires_isolation_first(&self) -> bool {
        false
    }
}

impl TransactionCapable for RuntimeConnection {}
