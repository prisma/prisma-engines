use super::*;
use crate::ast::*;

/// A representation of an SQL database transaction. If not commited, a
/// transaction will be rolled back by default when dropped.
///
/// Currently does not support nesting, so starting a new transaction using the
/// transaction object will panic.
pub struct Transaction<'a> {
    pub(crate) inner: &'a mut dyn Queryable,
    done: bool,
}

impl<'a> Transaction<'a> {
    pub(crate) fn new(inner: &'a mut dyn Queryable) -> crate::Result<Self> {
        inner.raw_cmd("BEGIN")?;
        Ok(Self { inner, done: false })
    }

    /// Commit the changes to the database and consume the transaction.
    pub fn commit(mut self) -> crate::Result<()> {
        self.done = true;
        self.inner.raw_cmd("COMMIT")?;

        Ok(())
    }

    /// Rolls back the changes to the database.
    pub fn rollback(&mut self) -> crate::Result<()> {
        self.done = true;
        self.inner.raw_cmd("ROLLBACK")?;

        Ok(())
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.rollback();
        }
    }
}

impl<'a> Queryable for Transaction<'a> {
    fn execute(&mut self, q: Query) -> crate::Result<Option<Id>> {
        self.inner.execute(q)
    }

    fn query(&mut self, q: Query) -> crate::Result<ResultSet> {
        self.inner.query(q)
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<ResultSet> {
        self.inner.query_raw(sql, params)
    }

    fn execute_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<u64> {
        self.inner.execute_raw(sql, params)
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.inner.turn_off_fk_constraints()
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.inner.turn_on_fk_constraints()
    }

    fn empty_tables(&mut self, tables: Vec<Table>) -> crate::Result<()> {
        self.inner.empty_tables(tables)
    }

    fn start_transaction<'b>(&'b mut self) -> crate::Result<Transaction<'b>> {
        panic!("Nested transactions are not supported")
    }

    fn raw_cmd(&mut self, cmd: &str) -> crate::Result<()> {
        self.inner.raw_cmd(cmd)
    }
}
