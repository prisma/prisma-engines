use super::*;
use crate::{
    ast::*,
    error::{Error, ErrorKind},
};
use async_trait::async_trait;
use metrics::{decrement_gauge, increment_gauge};
use std::{fmt, str::FromStr};

extern crate metrics as metrics;

#[async_trait]
pub trait Transaction: Queryable {
    /// Commit the changes to the database and consume the transaction.
    async fn commit(&self) -> crate::Result<()>;

    /// Rolls back the changes to the database.
    async fn rollback(&self) -> crate::Result<()>;

    /// workaround for lack of upcasting between traits https://github.com/rust-lang/rust/issues/65991
    fn as_queryable(&self) -> &dyn Queryable;
}

pub(crate) struct TransactionOptions {
    /// The isolation level to use.
    pub(crate) isolation_level: Option<IsolationLevel>,

    /// Whether or not to put the isolation level `SET` before or after the `BEGIN`.
    pub(crate) isolation_first: bool,
}

/// A default representation of an SQL database transaction. If not commited, a
/// transaction will be rolled back by default when dropped.
///
/// Currently does not support nesting, so starting a new transaction using the
/// transaction object will panic.
pub struct DefaultTransaction<'a> {
    pub inner: &'a dyn Queryable,
}

impl<'a> DefaultTransaction<'a> {
    pub(crate) async fn new(
        inner: &'a dyn Queryable,
        begin_stmt: &str,
        tx_opts: TransactionOptions,
    ) -> crate::Result<DefaultTransaction<'a>> {
        let this = Self { inner };

        if tx_opts.isolation_first {
            if let Some(isolation) = tx_opts.isolation_level {
                inner.set_tx_isolation_level(isolation).await?;
            }
        }

        inner.raw_cmd(begin_stmt).await?;

        if !tx_opts.isolation_first {
            if let Some(isolation) = tx_opts.isolation_level {
                inner.set_tx_isolation_level(isolation).await?;
            }
        }

        inner.server_reset_query(&this).await?;

        increment_gauge!("prisma_client_queries_active", 1.0);
        Ok(this)
    }
}

#[async_trait]
impl<'a> Transaction for DefaultTransaction<'a> {
    /// Commit the changes to the database and consume the transaction.
    async fn commit(&self) -> crate::Result<()> {
        decrement_gauge!("prisma_client_queries_active", 1.0);
        self.inner.raw_cmd("COMMIT").await?;

        Ok(())
    }

    /// Rolls back the changes to the database.
    async fn rollback(&self) -> crate::Result<()> {
        decrement_gauge!("prisma_client_queries_active", 1.0);
        self.inner.raw_cmd("ROLLBACK").await?;

        Ok(())
    }

    fn as_queryable(&self) -> &dyn Queryable {
        self
    }
}

#[async_trait]
impl<'a> Queryable for DefaultTransaction<'a> {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        self.inner.query(q).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        self.inner.execute(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.inner.query_raw_typed(sql, params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.inner.execute_raw_typed(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        self.inner.raw_cmd(cmd).await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        self.inner.version().await
    }

    fn is_healthy(&self) -> bool {
        self.inner.is_healthy()
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        self.inner.set_tx_isolation_level(isolation_level).await
    }

    fn requires_isolation_first(&self) -> bool {
        self.inner.requires_isolation_first()
    }
}

#[derive(Debug, Clone, Copy)]
/// Controls the locking and row versioning behavior of connections or transactions.
/// The levels correspond to the ANSI standard isolation levels, plus `Snapshot` for SQL Server.
///
/// Details on exact behavior and validity can be found in the documentation of the database vendors:
/// - [SQL Server documentation].
/// - [Postgres documentation].
/// - [MySQL documentation].
/// - [SQLite documentation].
///
/// [SQL Server documentation]: https://docs.microsoft.com/en-us/sql/t-sql/statements/set-transaction-isolation-level-transact-sql?view=sql-server-ver15
/// [Postgres documentation]: https://www.postgresql.org/docs/current/sql-set-transaction.html
/// [MySQL documentation]: https://dev.mysql.com/doc/refman/8.0/en/innodb-transaction-isolation-levels.html
/// [SQLite documentation]: https://www.sqlite.org/isolation.html
///
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Snapshot,
    Serializable,
}

impl fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadUncommitted => write!(f, "READ UNCOMMITTED"),
            Self::ReadCommitted => write!(f, "READ COMMITTED"),
            Self::RepeatableRead => write!(f, "REPEATABLE READ"),
            Self::Snapshot => write!(f, "SNAPSHOT"),
            Self::Serializable => write!(f, "SERIALIZABLE"),
        }
    }
}

impl FromStr for IsolationLevel {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s.to_lowercase().as_str() {
            "read uncommitted" | "readuncommitted" => Ok(Self::ReadUncommitted),
            "read committed" | "readcommitted" => Ok(Self::ReadCommitted),
            "repeatable read" | "repeatableread" => Ok(Self::RepeatableRead),
            "snapshot" => Ok(Self::Snapshot),
            "serializable" => Ok(Self::Serializable),
            _ => {
                let kind = ErrorKind::conversion(format!("Invalid isolation level `{s}`"));
                Err(Error::builder(kind).build())
            }
        }
    }
}
impl TransactionOptions {
    pub fn new(isolation_level: Option<IsolationLevel>, isolation_first: bool) -> Self {
        Self {
            isolation_level,
            isolation_first,
        }
    }
}
