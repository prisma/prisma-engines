use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{
        queryable::{ToColumnNames, ToRow},
        ResultSet,
    },
    visitor::{self, Visitor},
};
use rusqlite::Connection as SqliteConnection;

pub(crate) fn execute<'a>(conn: &SqliteConnection, q: Query<'a>) -> crate::Result<Option<Id>> {
    let (sql, params) = dbg!(visitor::Sqlite::build(q));

    let mut stmt = conn.prepare_cached(&sql)?;
    stmt.execute(params)?;

    Ok(Some(Id::Int(conn.last_insert_rowid() as usize)))
}

pub(crate) fn query<'a>(conn: &SqliteConnection, q: Query<'a>) -> crate::Result<ResultSet> {
    let (sql, params) = dbg!(visitor::Sqlite::build(q));
    query_raw(conn, &sql, &params)
}

pub(crate) fn query_raw<'a>(
    conn: &SqliteConnection,
    sql: &str,
    params: &[ParameterizedValue<'a>],
) -> crate::Result<ResultSet> {
    let mut stmt = conn.prepare_cached(sql)?;
    let mut rows = stmt.query(params)?;

    let mut result = ResultSet::new(rows.to_column_names(), Vec::new());

    while let Some(row) = rows.next()? {
        result.rows.push(row.to_result_row()?);
    }

    Ok(result)
}
