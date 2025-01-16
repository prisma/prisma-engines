pub mod expression;
pub mod translate;

use std::sync::Arc;

pub use expression::Expression;
use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    visitor::{Mssql, Mysql, Postgres, Sqlite},
};
use schema::QuerySchema;
use sql_query_builder::{Context, SqlQueryBuilder};
use thiserror::Error;
pub use translate::{translate, TranslateError};

use crate::{QueryDocument, QueryGraphBuilder};

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("only a single query can be compiled at a time")]
    UnsupportedRequest,

    #[error("{0}")]
    TranslateError(#[from] TranslateError),
}

pub fn compile(
    query_schema: &Arc<QuerySchema>,
    query_doc: QueryDocument,
    connection_info: &ConnectionInfo,
) -> crate::Result<Expression> {
    let QueryDocument::Single(query) = query_doc else {
        return Err(CompileError::UnsupportedRequest.into());
    };

    let ctx = Context::new(connection_info, None);
    let (graph, _serializer) = QueryGraphBuilder::new(query_schema).build(query)?;
    let res = match connection_info.sql_family() {
        SqlFamily::Postgres => translate(graph, &SqlQueryBuilder::<Postgres<'_>>::new(ctx)),
        SqlFamily::Mysql => translate(graph, &SqlQueryBuilder::<Mysql<'_>>::new(ctx)),
        SqlFamily::Sqlite => translate(graph, &SqlQueryBuilder::<Sqlite<'_>>::new(ctx)),
        SqlFamily::Mssql => translate(graph, &SqlQueryBuilder::<Mssql<'_>>::new(ctx)),
    };

    Ok(res.map_err(CompileError::TranslateError)?)
}
