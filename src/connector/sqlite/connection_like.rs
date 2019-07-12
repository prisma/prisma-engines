use super::PooledConnection;
use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{
        queryable::{Queryable, ToColumnNames, ToRow},
        ResultSet,
    },
    error::Error,
    visitor::{self, Visitor},
};
use std::convert::TryFrom;

pub enum ConnectionLike<'a> {
    Pooled(PooledConnection),
    Connection(rusqlite::Connection),
    Transaction(rusqlite::Transaction<'a>),
}

impl<'a> From<PooledConnection> for ConnectionLike<'a> {
    fn from(conn: PooledConnection) -> Self {
        ConnectionLike::Pooled(conn)
    }
}

impl<'a> From<rusqlite::Connection> for ConnectionLike<'a> {
    fn from(conn: rusqlite::Connection) -> Self {
        ConnectionLike::Connection(conn)
    }
}

impl<'a> From<rusqlite::Transaction<'a>> for ConnectionLike<'a> {
    fn from(conn: rusqlite::Transaction<'a>) -> Self {
        ConnectionLike::Transaction(conn)
    }
}

impl<'a> TryFrom<ConnectionLike<'a>> for rusqlite::Transaction<'a> {
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

impl<'a> TryFrom<ConnectionLike<'a>> for rusqlite::Connection {
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
    fn prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement> {
        match self {
            ConnectionLike::Pooled(c) => c.prepare_cached(sql),
            ConnectionLike::Connection(c) => c.prepare_cached(sql),
            ConnectionLike::Transaction(c) => c.prepare_cached(sql),
        }
    }

    fn last_insert_rowid(&self) -> i64 {
        match self {
            ConnectionLike::Pooled(c) => c.last_insert_rowid(),
            ConnectionLike::Connection(c) => c.last_insert_rowid(),
            ConnectionLike::Transaction(c) => c.last_insert_rowid(),
        }
    }

    pub fn transaction(&mut self) -> rusqlite::Result<rusqlite::Transaction> {
        match self {
            ConnectionLike::Pooled(ref mut c) => c.transaction(),
            ConnectionLike::Connection(ref mut c) => c.transaction(),
            ConnectionLike::Transaction(_) => {
                panic!("Could not start a transaction from transaction")
            }
        }
    }
}

impl<'t> Queryable for ConnectionLike<'t> {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Sqlite::build(q));

        self.execute_raw(&sql, &params)?;

        Ok(Some(Id::Int(self.last_insert_rowid() as usize)))
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Sqlite::build(q));
        self.query_raw(&sql, &params)
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        let mut stmt = self.prepare_cached(sql)?;
        let mut rows = stmt.query(params)?;

        let mut result = ResultSet::new(rows.to_column_names(), Vec::new());

        while let Some(row) = rows.next()? {
            result.rows.push(row.to_result_row()?);
        }

        Ok(result)
    }

    fn execute_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<u64> {
        let mut stmt = self.prepare_cached(sql)?;
        let changes = stmt.execute(params)?;

        Ok(u64::try_from(changes).unwrap())
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("PRAGMA foreign_keys = OFF", &[])?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("PRAGMA foreign_keys = ON", &[])?;
        Ok(())
    }
}
