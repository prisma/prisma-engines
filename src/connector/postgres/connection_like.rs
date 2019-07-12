use super::conversion;
use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet},
    error::Error,
    visitor::{self, Visitor},
};
use postgres::{
    types::{FromSql, ToSql},
    Statement,
};
use std::convert::TryFrom;

type PooledConnection = r2d2::PooledConnection<super::Manager>;

pub enum ConnectionLike<'a> {
    Pooled(PooledConnection),
    Connection(postgres::Client),
    Transaction(postgres::Transaction<'a>),
}

impl<'a> From<PooledConnection> for ConnectionLike<'a> {
    fn from(conn: PooledConnection) -> Self {
        ConnectionLike::Pooled(conn)
    }
}

impl<'a> From<postgres::Client> for ConnectionLike<'a> {
    fn from(conn: postgres::Client) -> Self {
        ConnectionLike::Connection(conn)
    }
}

impl<'a> From<postgres::Transaction<'a>> for ConnectionLike<'a> {
    fn from(conn: postgres::Transaction<'a>) -> Self {
        ConnectionLike::Transaction(conn)
    }
}

impl<'a> TryFrom<ConnectionLike<'a>> for postgres::Transaction<'a> {
    type Error = Error;

    fn try_from(cl: ConnectionLike<'a>) -> crate::Result<Self> {
        match cl {
            ConnectionLike::Transaction(tx) => Ok(tx),
            _ => Err(Error::ConversionError(
                "ConnectionLike was not a transaction...",
            )),
        }
    }
}

impl<'a> TryFrom<ConnectionLike<'a>> for PooledConnection {
    type Error = Error;

    fn try_from(cl: ConnectionLike<'a>) -> crate::Result<Self> {
        match cl {
            ConnectionLike::Pooled(pooled) => Ok(pooled),
            _ => Err(Error::ConversionError(
                "ConnectionLike was not a pooled connection...",
            )),
        }
    }
}

impl<'a> TryFrom<ConnectionLike<'a>> for postgres::Client {
    type Error = Error;

    fn try_from(cl: ConnectionLike<'a>) -> crate::Result<Self> {
        match cl {
            ConnectionLike::Connection(conn) => Ok(conn),
            _ => Err(Error::ConversionError(
                "ConnectionLike was not a connection...",
            )),
        }
    }
}

impl<'a> ConnectionLike<'a> {
    pub fn query<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<Vec<tokio_postgres::row::Row>, tokio_postgres::error::Error>
    where
        T: postgres::ToStatement,
    {
        match self {
            ConnectionLike::Pooled(ref mut conn) => conn.query(query, params),
            ConnectionLike::Connection(ref mut conn) => conn.query(query, params),
            ConnectionLike::Transaction(ref mut conn) => conn.query(query, params),
        }
    }

    pub fn prepare(&mut self, query: &str) -> Result<Statement, tokio_postgres::error::Error> {
        match self {
            ConnectionLike::Pooled(ref mut conn) => conn.prepare(query),
            ConnectionLike::Connection(ref mut conn) => conn.prepare(query),
            ConnectionLike::Transaction(ref mut conn) => conn.prepare(query),
        }
    }
}

impl<'t> Queryable for ConnectionLike<'t> {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        let stmt = self.prepare(&sql)?;
        let rows = self.query(&stmt, &conversion::conv_params(&params))?;

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
        let stmt = self.prepare(&sql)?;
        let rows = self.query(&stmt, &conversion::conv_params(params))?;

        let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());

        for row in rows {
            result.rows.push(row.to_result_row()?);
        }

        Ok(result)
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET CONSTRAINTS ALL DEFERRED", &[])?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET CONSTRAINTS ALL IMMEDIATE", &[])?;
        Ok(())
    }
}
