use super::conversion;
use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet},
    visitor::{self, Visitor},
};
use postgres::{
    types::{FromSql, ToSql},
    Statement,
};

pub struct ConnectionLike<T>
where
    T: LikePsqlConnection,
{
    inner: T,
}

impl From<super::PostgreSql> for ConnectionLike<super::PostgreSql> {
    fn from(inner: super::PostgreSql) -> Self {
        ConnectionLike { inner }
    }
}

impl<'a> From<postgres::Transaction<'a>> for ConnectionLike<postgres::Transaction<'a>> {
    fn from(inner: postgres::Transaction<'a>) -> Self {
        ConnectionLike { inner }
    }
}

pub trait LikePsqlConnection {
    fn query<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<Vec<postgres::row::Row>, postgres::error::Error>
    where
        T: postgres::ToStatement;

    fn prepare(&mut self, query: &str) -> Result<Statement, postgres::error::Error>;

    fn _execute<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<u64, postgres::error::Error>
    where
        T: postgres::ToStatement;

    fn start_transaction<'a>(&'a mut self)
        -> Result<postgres::Transaction, postgres::error::Error>;
}

impl LikePsqlConnection for super::PostgreSql {
    fn query<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<Vec<postgres::row::Row>, postgres::error::Error>
    where
        T: postgres::ToStatement,
    {
        self.client.query(query, params)
    }

    fn prepare(&mut self, query: &str) -> Result<Statement, postgres::error::Error> {
        self.client.prepare(query)
    }

    fn _execute<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<u64, postgres::error::Error>
    where
        T: postgres::ToStatement,
    {
        self.client.execute(query, params)
    }

    fn start_transaction<'a>(
        &'a mut self,
    ) -> Result<postgres::Transaction, postgres::error::Error> {
        self.client.transaction()
    }
}

impl<'t> LikePsqlConnection for postgres::Transaction<'t> {
    fn query<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<Vec<postgres::row::Row>, postgres::error::Error>
    where
        T: postgres::ToStatement,
    {
        self.query(query, params)
    }

    fn prepare(&mut self, query: &str) -> Result<Statement, postgres::error::Error> {
        self.prepare(query)
    }

    fn _execute<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<u64, postgres::error::Error>
    where
        T: postgres::ToStatement,
    {
        self.execute(query, params)
    }

    fn start_transaction<'a>(
        &'a mut self,
    ) -> Result<postgres::Transaction, postgres::error::Error> {
        panic!("Nested transactions are not supported for PostgreSQL")
    }
}

impl<C> Queryable for ConnectionLike<C>
where
    C: LikePsqlConnection,
{
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        let stmt = self.inner.prepare(&sql)?;
        let rows = self.inner.query(&stmt, &conversion::conv_params(&params))?;

        let id = rows.into_iter().rev().next().map(|row| {
            let id = row.get(0);
            let tpe = row.columns()[0].type_();

            Id::from_sql(tpe, id)
        });

        match id {
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(_)) => panic!("Cannot convert err, todo."),
            None => Ok(None),
        }
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));
        self.query_raw(sql.as_str(), &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        let stmt = self.inner.prepare(&sql)?;
        let rows = self.inner.query(&stmt, &conversion::conv_params(params))?;

        let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());

        for row in rows {
            result.rows.push(row.to_result_row()?);
        }

        Ok(result)
    }

    fn execute_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<u64> {
        let stmt = self.inner.prepare(&sql)?;
        let changes = self.inner._execute(&stmt, &conversion::conv_params(params))?;

        Ok(changes)
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET CONSTRAINTS ALL DEFERRED", &[])?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET CONSTRAINTS ALL IMMEDIATE", &[])?;
        Ok(())
    }

    fn start_transaction<'a>(&'a mut self) -> crate::Result<Box<dyn Transaction + 'a>> {
        let tx = ConnectionLike::from(self.inner.start_transaction()?);
        Ok(Box::new(tx))
    }
}

impl<'t> Transaction for ConnectionLike<postgres::Transaction<'t>> {
    fn commit(self) -> crate::Result<()> {
        Ok(self.inner.commit()?)
    }

    fn rollback(self) -> crate::Result<()> {
        Ok(self.inner.rollback()?)
    }
}
