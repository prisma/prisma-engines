use super::conversion;
use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet},
    error::Error,
    visitor::{self, Visitor},
};
use mysql as my;
use std::convert::TryFrom;

type PooledConnection = r2d2::PooledConnection<super::Manager>;

pub enum ConnectionLike<'a> {
    Pooled(PooledConnection),
    Connection(my::Conn),
    Transaction(my::Transaction<'a>),
}

impl<'a> From<PooledConnection> for ConnectionLike<'a> {
    fn from(conn: PooledConnection) -> Self {
        ConnectionLike::Pooled(conn)
    }
}

impl<'a> From<my::Conn> for ConnectionLike<'a> {
    fn from(conn: my::Conn) -> Self {
        ConnectionLike::Connection(conn)
    }
}

impl<'a> From<my::Transaction<'a>> for ConnectionLike<'a> {
    fn from(conn: my::Transaction<'a>) -> Self {
        ConnectionLike::Transaction(conn)
    }
}

impl<'a> TryFrom<ConnectionLike<'a>> for my::Transaction<'a> {
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

impl<'a> TryFrom<ConnectionLike<'a>> for my::Conn {
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
    pub fn prepare<T: AsRef<str>>(&mut self, query: T) -> my::Result<my::Stmt> {
        match self {
            ConnectionLike::Pooled(ref mut conn) => conn.prepare(query),
            ConnectionLike::Connection(ref mut conn) => conn.prepare(query),
            ConnectionLike::Transaction(ref mut conn) => conn.prepare(query),
        }
    }
}

impl<'t> Queryable for ConnectionLike<'t> {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));

        let mut stmt = self.prepare(&sql)?;
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
        let mut stmt = self.prepare(&sql)?;
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
        let mut stmt = self.prepare(sql)?;
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
}
