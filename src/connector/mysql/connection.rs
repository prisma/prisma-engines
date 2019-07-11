use crate::{
    ast::*,
    connector::{
        transaction::{ToColumnNames, ToRow},
        ResultSet,
    },
    visitor::{self, Visitor},
};
use mysql::{self as my, params::Params, Value};

pub trait GenericConnection {
    fn prepared<T: AsRef<str>>(&mut self, query: T) -> my::Result<my::Stmt>;
}

impl<'a> GenericConnection for my::Transaction<'a> {
    fn prepared<T: AsRef<str>>(&mut self, query: T) -> my::Result<my::Stmt> {
        self.prepare(query)
    }
}

impl GenericConnection for super::PooledConnection {
    fn prepared<T: AsRef<str>>(&mut self, query: T) -> my::Result<my::Stmt> {
        self.prepare(query)
    }
}

pub(crate) fn execute<'a, C>(conn: &mut C, q: Query<'a>) -> crate::Result<Option<Id>>
where
    C: GenericConnection,
{
    let (sql, params) = dbg!(visitor::Mysql::build(q));

    let mut stmt = conn.prepared(&sql)?;
    let result = stmt.execute(params)?;

    Ok(Some(Id::from(result.last_insert_id())))
}

pub(crate) fn query<'a, C>(conn: &mut C, q: Query<'a>) -> crate::Result<ResultSet>
where
    C: GenericConnection,
{
    let (sql, params) = dbg!(visitor::Mysql::build(q));
    query_raw(conn, &sql, &params[..])
}

pub(crate) fn query_raw<'a, C>(
    conn: &mut C,
    sql: &str,
    params: &[ParameterizedValue<'a>],
) -> crate::Result<ResultSet>
where
    C: GenericConnection,
{
    let mut stmt = conn.prepared(&sql)?;
    let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());
    let rows = stmt.execute(conv_params(params))?;

    for row in rows {
        result.rows.push(row?.to_result_row()?);
    }

    Ok(result)
}

fn conv_params<'a>(params: &[ParameterizedValue<'a>]) -> Params {
    if params.len() > 0 {
        Params::Positional(params.iter().map(|x| x.into()).collect::<Vec<Value>>())
    } else {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        Params::Empty
    }
}
