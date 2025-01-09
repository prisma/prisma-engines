use async_trait::async_trait;
use prisma_metrics::guards::GaugeGuard;

use super::*;
use crate::{
    ast::*,
    error::{Error, ErrorKind},
};
use std::{
    fmt,
    str::FromStr,
    sync::atomic::{AtomicU32, Ordering},
};

#[async_trait]
pub trait Transaction: Queryable {
    fn depth(&self) -> u32;

    /// Start a new transaction or nested transaction via savepoint.
    async fn begin(&self) -> crate::Result<()>;

    /// Commit the changes to the database and consume the transaction.
    async fn commit(&self) -> crate::Result<()>;

    /// Rolls back the changes to the database.
    async fn rollback(&self) -> crate::Result<()>;

    /// Creates a savepoint in the transaction.
    async fn create_savepoint(&self) -> crate::Result<()>;

    /// Releases a savepoint in the transaction.
    async fn release_savepoint(&self) -> crate::Result<()>;

    /// Rolls back to a savepoint in the transaction.
    async fn rollback_to_savepoint(&self) -> crate::Result<()>;

    /// workaround for lack of upcasting between traits https://github.com/rust-lang/rust/issues/65991
    fn as_queryable(&self) -> &dyn Queryable;
}

#[cfg(any(
    feature = "sqlite-native",
    feature = "mssql-native",
    feature = "postgresql-native",
    feature = "mysql-native"
))]
pub(crate) struct TransactionOptions {
    /// The isolation level to use.
    pub(crate) isolation_level: Option<IsolationLevel>,

    /// Whether or not to put the isolation level `SET` before or after the `BEGIN`.
    pub(crate) isolation_first: bool,
}

#[cfg(any(
    feature = "sqlite-native",
    feature = "mssql-native",
    feature = "postgresql-native",
    feature = "mysql-native"
))]
impl TransactionOptions {
    pub fn new(isolation_level: Option<IsolationLevel>, isolation_first: bool) -> Self {
        Self {
            isolation_level,
            isolation_first,
        }
    }
}

/// A default representation of an SQL database transaction. If not commited, a
/// transaction will be rolled back by default when dropped.
///
/// Currently does not support nesting, so starting a new transaction using the
/// transaction object will panic.
pub struct DefaultTransaction<'a> {
    pub inner: &'a dyn Queryable,
    pub depth: AtomicU32,
    gauge: GaugeGuard,
}

#[cfg_attr(
    not(any(
        feature = "sqlite-native",
        feature = "mssql-native",
        feature = "postgresql-native",
        feature = "mysql-native"
    )),
    allow(clippy::needless_lifetimes)
)]
impl<'a> DefaultTransaction<'a> {
    #[cfg(any(
        feature = "sqlite-native",
        feature = "mssql-native",
        feature = "postgresql-native",
        feature = "mysql-native"
    ))]
    pub(crate) async fn new(
        inner: &'a dyn Queryable,
        tx_opts: TransactionOptions,
    ) -> crate::Result<DefaultTransaction<'a>> {
        let this = Self {
            inner,
            gauge: GaugeGuard::increment("prisma_client_queries_active"),
            depth: AtomicU32::new(0),
        };

        if tx_opts.isolation_first {
            if let Some(isolation) = tx_opts.isolation_level {
                inner.set_tx_isolation_level(isolation).await?;
            }
        }

        this.begin().await?;

        if !tx_opts.isolation_first {
            if let Some(isolation) = tx_opts.isolation_level {
                inner.set_tx_isolation_level(isolation).await?;
            }
        }

        inner.server_reset_query(&this).await?;

        Ok(this)
    }
}

#[async_trait]
impl Transaction for DefaultTransaction<'_> {
    fn depth(&self) -> u32 {
        self.depth.load(Ordering::Relaxed)
    }

    async fn begin(&self) -> crate::Result<()> {
        self.depth.fetch_add(1, Ordering::Relaxed);

        let begin_statement = self.inner.begin_statement();

        self.inner.raw_cmd(begin_statement).await?;

        Ok(())
    }

    /// Commit the changes to the database and consume the transaction.
    async fn commit(&self) -> crate::Result<()> {
        // Perform the asynchronous operation without holding the lock
        self.inner.raw_cmd("COMMIT").await?;

        self.depth.fetch_sub(1, Ordering::Relaxed);

        self.gauge.decrement();

        Ok(())
    }

    /// Rolls back the changes to the database.
    async fn rollback(&self) -> crate::Result<()> {
        self.inner.raw_cmd("ROLLBACK").await?;

        self.depth.fetch_sub(1, Ordering::Relaxed);

        self.gauge.decrement();

        Ok(())
    }

    /// Creates a savepoint in the transaction
    async fn create_savepoint(&self) -> crate::Result<()> {
        let new_depth = self.depth.fetch_add(1, Ordering::Relaxed) + 1;

        let create_savepoint_statement = self.inner.create_savepoint_statement(new_depth);
        self.inner.raw_cmd(&create_savepoint_statement).await?;
        Ok(())
    }

    /// Releases a savepoint in the transaction
    async fn release_savepoint(&self) -> crate::Result<()> {
        let depth_val = self.depth.load(Ordering::Relaxed);

        if depth_val == 0 {
            panic!(
                "No savepoint to release in transaction, make sure to call create_savepoint before release_savepoint"
            );
        }

        // Perform the asynchronous operation without holding the lock
        let release_savepoint_statement = self.inner.release_savepoint_statement(depth_val);
        self.inner.raw_cmd(&release_savepoint_statement).await?;

        self.depth.fetch_sub(1, Ordering::Relaxed);

        Ok(())
    }

    /// Rollback to savepoint in the transaction
    async fn rollback_to_savepoint(&self) -> crate::Result<()> {
        let depth_val = self.depth.load(Ordering::Relaxed);

        if depth_val == 0 {
            panic!(
                "No savepoint to rollback to in transaction, make sure to call create_savepoint before rollback_to_savepoint"
            );
        }

        let rollback_to_savepoint_statement = self.inner.rollback_to_savepoint_statement(depth_val);
        self.inner.raw_cmd(&rollback_to_savepoint_statement).await?;

        self.depth.fetch_sub(1, Ordering::Relaxed);

        Ok(())
    }

    fn as_queryable(&self) -> &dyn Queryable {
        self
    }
}

#[async_trait]
impl Queryable for DefaultTransaction<'_> {
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

    async fn describe_query(&self, sql: &str) -> crate::Result<DescribedQuery> {
        self.inner.describe_query(sql).await
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
