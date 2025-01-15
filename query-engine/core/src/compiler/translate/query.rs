mod convert;
mod read;
mod write;

use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    visitor::Visitor,
};
use query_builder::DbQuery;
use read::translate_read_query;
use sql_query_builder::Context;
use write::translate_write_query;

use crate::{compiler::expression::Expression, Query};

use super::TranslateResult;

pub(crate) fn translate_query(query: Query, connection_info: &ConnectionInfo) -> TranslateResult<Expression> {
    let ctx = Context::new(connection_info, None);

    match query {
        Query::Read(rq) => translate_read_query(rq, &ctx),
        Query::Write(wq) => translate_write_query(wq, &ctx),
    }
}

fn build_db_query<'a>(query: impl Into<quaint::ast::Query<'a>>, ctx: &Context<'_>) -> TranslateResult<DbQuery> {
    let (sql, params) = match ctx.connection_info.sql_family() {
        SqlFamily::Postgres => quaint::visitor::Postgres::build(query)?,
        // TODO: implement proper switch for other databases once proper feature flags are supported/logic is extracted
        _ => unimplemented!(),
        // SqlFamily::Mysql => quaint::visitor::Mysql::build(query)?,
        // SqlFamily::Sqlite => quaint::visitor::Sqlite::build(query)?,
        // SqlFamily::Mssql => quaint::visitor::Mssql::build(query)?,
    };

    let params = params
        .into_iter()
        .map(convert::quaint_value_to_prisma_value)
        .collect::<Vec<_>>();
    Ok(DbQuery::new(sql, params))
}
