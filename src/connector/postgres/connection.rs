use crate::{
    ast::*,
    connector::{
        transaction::{ToColumnNames, ToRow},
        ResultSet,
    },
    visitor::{self, Visitor},
};
use postgres::{
    types::{FromSql, ToSql},
    Client, Statement, ToStatement, Transaction,
};

pub trait GenericConnection {
    fn _query<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<Vec<tokio_postgres::row::Row>, tokio_postgres::error::Error>
    where
        T: ToStatement;

    fn _prepare(&mut self, query: &str) -> Result<Statement, tokio_postgres::error::Error>;
}

impl<'a> GenericConnection for Transaction<'a> {
    fn _query<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<Vec<tokio_postgres::row::Row>, tokio_postgres::error::Error>
    where
        T: ToStatement,
    {
        self.query(query, params)
    }

    fn _prepare(&mut self, query: &str) -> Result<Statement, tokio_postgres::error::Error> {
        self.prepare(query)
    }
}

impl GenericConnection for &mut Client {
    fn _query<T: ?Sized>(
        &mut self,
        query: &T,
        params: &[&dyn ToSql],
    ) -> Result<Vec<tokio_postgres::row::Row>, tokio_postgres::error::Error>
    where
        T: ToStatement,
    {
        self.query(query, params)
    }

    fn _prepare(&mut self, query: &str) -> Result<Statement, tokio_postgres::error::Error> {
        self.prepare(query)
    }
}

pub(crate) fn execute<'a, C>(conn: &mut C, q: Query<'a>) -> crate::Result<Option<Id>>
where
    C: GenericConnection,
{
    let (sql, params) = dbg!(visitor::Postgres::build(q));

    let stmt = conn._prepare(&sql)?;
    let rows = conn._query(&stmt, &conv_params(&params))?;

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

pub(crate) fn query<'a, C>(conn: &mut C, q: Query<'a>) -> crate::Result<ResultSet>
where
    C: GenericConnection,
{
    let (sql, params) = dbg!(visitor::Postgres::build(q));
    query_raw(conn, sql.as_str(), &params[..])
}

pub(crate) fn query_raw<'a, C>(
    conn: &mut C,
    sql: &str,
    params: &[ParameterizedValue<'a>],
) -> crate::Result<ResultSet>
where
    C: GenericConnection,
{
    let stmt = conn._prepare(&sql)?;
    let rows = conn._query(&stmt, &conv_params(params))?;

    let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());

    for row in rows {
        result.rows.push(row.to_result_row()?);
    }

    Ok(result)
}

fn conv_params<'a>(params: &'a [ParameterizedValue<'a>]) -> Vec<&'a tokio_postgres::types::ToSql> {
    params.into_iter().map(|x| x as &ToSql).collect::<Vec<_>>()
}
