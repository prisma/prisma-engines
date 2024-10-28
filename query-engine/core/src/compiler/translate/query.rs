mod convert;
mod read;
mod write;

use quaint::{
    prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily},
    visitor::Visitor,
};
use read::translate_read_query;
use sql_query_connector::context::Context;
use write::translate_write_query;

use crate::{
    compiler::expression::{DbQuery, Expression},
    Query,
};

use super::TranslateResult;

pub(crate) fn translate_query(query: Query) -> TranslateResult<Expression> {
    let connection_info =
        ConnectionInfo::External(ExternalConnectionInfo::new(SqlFamily::Sqlite, "main".to_owned(), None));

    let ctx = Context::new(&connection_info, None);

    match query {
        Query::Read(rq) => translate_read_query(rq, &ctx),
        Query::Write(wq) => translate_write_query(wq, &ctx),
    }
}

fn build_db_query<'a>(query: impl Into<quaint::ast::Query<'a>>) -> TranslateResult<DbQuery> {
    let (sql, params) = quaint::visitor::Postgres::build(query)?;
    let params = params
        .into_iter()
        .map(convert::quaint_value_to_prisma_value)
        .collect::<Vec<_>>();
    Ok(DbQuery::new(sql, params))
}
