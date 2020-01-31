use super::*;
use crate::ast::*;

/// A representation of an SQL database transaction. If not commited, a
/// transaction will be rolled back by default when dropped.
///
/// Currently does not support nesting, so starting a new transaction using the
/// transaction object will panic.
pub struct Transaction<'a> {
    pub(crate) inner: &'a dyn Queryable,
}

impl<'a> Transaction<'a> {
    pub(crate) async fn new(inner: &'a dyn Queryable) -> crate::Result<Transaction<'a>> {
        inner.raw_cmd("BEGIN").await?;
        Ok(Self { inner })
    }

    /// Commit the changes to the database and consume the transaction.
    pub async fn commit(&self) -> crate::Result<()> {
        self.inner.raw_cmd("COMMIT").await?;

        Ok(())
    }

    /// Rolls back the changes to the database.
    pub async fn rollback(&self) -> crate::Result<()> {
        self.inner.raw_cmd("ROLLBACK").await?;

        Ok(())
    }
}

impl<'a> Queryable for Transaction<'a> {
    fn query<'b>(&'b self, q: Query<'b>) -> DBIO<'b, ResultSet> {
        self.inner.query(q)
    }

    fn execute<'b>(&'b self, q: Query<'b>) -> DBIO<'b, u64> {
        self.inner.execute(q)
    }

    fn query_raw<'b>(&'b self, sql: &'b str, params: &'b [ParameterizedValue]) -> DBIO<'b, ResultSet> {
        self.inner.query_raw(sql, params)
    }

    fn execute_raw<'b>(&'b self, sql: &'b str, params: &'b [ParameterizedValue<'b>]) -> DBIO<'b, u64> {
        self.inner.execute_raw(sql, params)
    }

    fn raw_cmd<'b>(&'b self, cmd: &'b str) -> DBIO<'b, ()> {
        self.inner.raw_cmd(cmd)
    }
}
