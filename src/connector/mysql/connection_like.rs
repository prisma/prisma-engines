use super::conversion;
use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet},
    visitor::{self, Visitor},
};
use mysql as my;

pub struct ConnectionLike<T>
where
    T: LikeMysqlConnection,
{
    inner: T,
}

impl From<super::Mysql> for ConnectionLike<super::Mysql> {
    fn from(inner: super::Mysql) -> Self {
        ConnectionLike { inner }
    }
}

impl<'a> From<my::Transaction<'a>> for ConnectionLike<my::Transaction<'a>> {
    fn from(inner: my::Transaction<'a>) -> Self {
        ConnectionLike { inner }
    }
}

pub trait LikeMysqlConnection {
    fn prepare<T: AsRef<str>>(&mut self, query: T) -> my::Result<my::Stmt>;
    fn start_transaction<'a>(&'a mut self) -> my::Result<my::Transaction>;
}

impl LikeMysqlConnection for super::Mysql {
    fn prepare<T: AsRef<str>>(&mut self, query: T) -> my::Result<my::Stmt> {
        self.client.prepare(query)
    }

    fn start_transaction<'a>(&'a mut self) -> my::Result<my::Transaction> {
        self.client.start_transaction(true, None, None)
    }
}

impl<'a> LikeMysqlConnection for my::Transaction<'a> {
    fn prepare<T: AsRef<str>>(&mut self, query: T) -> my::Result<my::Stmt> {
        self.prepare(query)
    }

    fn start_transaction(&mut self) -> my::Result<my::Transaction> {
        panic!("Nested transactions are not supported for MySQL")
    }
}

impl<C> Queryable for ConnectionLike<C> where C: LikeMysqlConnection {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));

        let mut stmt = self.inner.prepare(&sql)?;
        let result = stmt.execute(params)?;

        Ok(Some(Id::from(result.last_insert_id())))
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));
        self.query_raw(&sql, &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        let mut stmt = self.inner.prepare(&sql)?;
        let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());
        let rows = stmt.execute(conversion::conv_params(params))?;

        for row in rows {
            result.rows.push(row?.to_result_row()?);
        }

        Ok(result)
    }

    fn execute_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<u64> {
        let mut stmt = self.inner.prepare(sql)?;
        let result = stmt.execute(conversion::conv_params(params))?;

        Ok(result.affected_rows())
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET FOREIGN_KEY_CHECKS=0", &[])?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET FOREIGN_KEY_CHECKS=1", &[])?;
        Ok(())
    }

    fn start_transaction<'a>(&'a mut self) -> crate::Result<Box<dyn Transaction + 'a>> {
        let tx = ConnectionLike::from(self.inner.start_transaction()?);
        Ok(Box::new(tx))
    }
}

impl<'t> Transaction for ConnectionLike<my::Transaction<'t>> {
    fn commit(self) -> crate::Result<()> {
        Ok(self.inner.commit()?)
    }

    fn rollback(self) -> crate::Result<()> {
        Ok(self.inner.rollback()?)
    }
}
