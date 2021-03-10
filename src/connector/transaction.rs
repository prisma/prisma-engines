use super::*;
use crate::ast::*;
use async_trait::async_trait;

/// A representation of an SQL database transaction. If not commited, a
/// transaction will be rolled back by default when dropped.
///
/// Currently does not support nesting, so starting a new transaction using the
/// transaction object will panic.
pub struct Transaction<'a> {
    pub(crate) inner: &'a dyn Queryable,
}

impl<'a> Transaction<'a> {
    #[tracing::instrument(name = "new_transaction", skip(inner, begin_stmt))]
    pub(crate) async fn new(inner: &'a dyn Queryable, begin_stmt: &str) -> crate::Result<Transaction<'a>> {
        let this = Self { inner };

        inner.raw_cmd(begin_stmt).await?;
        inner.server_reset_query(&this).await?;

        Ok(this)
    }

    /// Commit the changes to the database and consume the transaction.
    #[tracing::instrument(skip(self))]
    pub async fn commit(&self) -> crate::Result<()> {
        self.inner.raw_cmd("COMMIT").await?;

        Ok(())
    }

    /// Rolls back the changes to the database.
    #[tracing::instrument(skip(self))]
    pub async fn rollback(&self) -> crate::Result<()> {
        self.inner.raw_cmd("ROLLBACK").await?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Queryable for Transaction<'a> {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        self.inner.query(q).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        self.inner.execute(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        self.inner.raw_cmd(cmd).await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        self.inner.version().await
    }
}
