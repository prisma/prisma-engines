use super::*;
use crate::ast::*;

/// A representation of an SQL database transaction. If not commited, a
/// transaction will be rolled back by default when dropped.
///
/// Currently does not support nesting, so starting a new transaction using the
/// transaction object will panic.
pub struct Transaction<'a> {
    pub(crate) inner: &'a dyn Queryable,
    done: bool,
}

impl<'a> Transaction<'a> {
    pub(crate) async fn new(inner: &'a dyn Queryable) -> crate::Result<Transaction<'a>> {
        inner.raw_cmd("BEGIN").await?;
        Ok(Self { inner, done: false })
    }

    /// Commit the changes to the database and consume the transaction.
    pub async fn commit(mut self) -> crate::Result<()> {
        self.done = true;
        self.inner.raw_cmd("COMMIT").await?;

        Ok(())
    }

    /// Rolls back the changes to the database.
    pub async fn rollback(&mut self) -> crate::Result<()> {
        self.done = true;
        self.inner.raw_cmd("ROLLBACK").await?;

        Ok(())
    }

    pub fn is_done(&self) -> bool {
        self.done
    }
}

impl<'a> Queryable for Transaction<'a> {
    fn execute<'b>(&'b self, q: Query<'b>) -> DBIO<'b, Option<Id>> {
        self.inner.execute(q)
    }

    fn query<'b>(&'b self, q: Query<'b>) -> DBIO<'b, ResultSet> {
        self.inner.query(q)
    }

    fn query_raw<'b>(&'b self, sql: &'b str, params: &'b [ParameterizedValue]) -> DBIO<'b, ResultSet> {
        self.inner.query_raw(sql, params)
    }

    fn execute_raw<'b>(&'b self, sql: &'b str, params: &'b [ParameterizedValue]) -> DBIO<'b, u64> {
        self.inner.execute_raw(sql, params)
    }

    fn turn_off_fk_constraints<'b>(&'b self) -> DBIO<'b, ()> {
        self.inner.turn_off_fk_constraints()
    }

    fn turn_on_fk_constraints<'b>(&'b self) -> DBIO<'b, ()> {
        self.inner.turn_on_fk_constraints()
    }

    fn empty_tables<'b>(&'b self, tables: Vec<Table<'b>>) -> DBIO<'b, ()> {
        self.inner.empty_tables(tables)
    }

    fn start_transaction<'b>(&'b self) -> DBIO<'b, Transaction<'b>> {
        panic!("Nested transactions are not supported")
    }

    fn raw_cmd<'b>(&'b self, cmd: &'b str) -> DBIO<'b, ()> {
        self.inner.raw_cmd(cmd)
    }
}
