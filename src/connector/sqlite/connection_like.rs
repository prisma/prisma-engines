use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{
        queryable::*,
        ResultSet,
    },
    visitor::{self, Visitor},
};
use std::convert::TryFrom;

pub struct ConnectionLike<T>
where
    T: LikeSqliteConnection,
{
    inner: T,
}

impl From<super::Sqlite> for ConnectionLike<super::Sqlite> {
    fn from(inner: super::Sqlite) -> Self {
        ConnectionLike { inner }
    }
}

impl<'a> From<rusqlite::Transaction<'a>> for ConnectionLike<rusqlite::Transaction<'a>> {
    fn from(inner: rusqlite::Transaction<'a>) -> Self {
        ConnectionLike { inner }
    }
}

pub trait LikeSqliteConnection {
    fn _prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement>;
    fn _last_insert_rowid(&self) -> i64;
    fn start_transaction<'a>(&'a mut self) -> rusqlite::Result<rusqlite::Transaction>;
}

impl LikeSqliteConnection for super::Sqlite {
    fn _prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement> {
        self.client.prepare_cached(sql)
    }

    fn _last_insert_rowid(&self) -> i64 {
        self.client.last_insert_rowid()
    }

    fn start_transaction<'a>(&'a mut self) -> rusqlite::Result<rusqlite::Transaction> {
        self.client.transaction()
    }
}

impl<'t> LikeSqliteConnection for rusqlite::Transaction<'t> {
    fn _prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement> {
        self.prepare_cached(sql)
    }

    fn _last_insert_rowid(&self) -> i64 {
        self.last_insert_rowid()
    }

    fn start_transaction(&mut self) -> rusqlite::Result<rusqlite::Transaction> {
        panic!("Nested transactions are not supported for MySQL")
    }
}

impl<C> Queryable for ConnectionLike<C> where C: LikeSqliteConnection {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Sqlite::build(q));
        self.execute_raw(&sql, &params)?;

        Ok(Some(Id::Int(self.inner._last_insert_rowid() as usize)))
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
        let mut stmt = self.inner._prepare_cached(sql)?;
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
        let mut stmt = self.inner._prepare_cached(sql)?;
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

    fn start_transaction<'a>(&'a mut self) -> crate::Result<Box<dyn Transaction + 'a>> {
        let tx = ConnectionLike::from(self.inner.start_transaction()?);
        Ok(Box::new(tx))
    }
}

impl<'t> Transaction for ConnectionLike<rusqlite::Transaction<'t>> {
    fn commit(self) -> crate::Result<()> {
        Ok(self.inner.commit()?)
    }

    fn rollback(self) -> crate::Result<()> {
        Ok(self.inner.rollback()?)
    }
}
