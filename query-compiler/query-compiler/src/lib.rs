pub mod expression;
pub mod translate;

use std::sync::Arc;

pub use expression::Expression;
use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    visitor,
};
use query_core::{schema::QuerySchema, QueryGraphBuilderError};
use sql_query_builder::{Context, SqlQueryBuilder};
use thiserror::Error;
pub use translate::{translate, TranslateError};

use query_core::{QueryDocument, QueryGraphBuilder};

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("only a single query can be compiled at a time")]
    UnsupportedRequest,

    #[error("failed to build query graph: {0}")]
    GraphBuildError(#[from] QueryGraphBuilderError),

    #[error("{0}")]
    TranslateError(#[from] TranslateError),
}

pub fn compile(
    query_schema: &Arc<QuerySchema>,
    query_doc: QueryDocument,
    connection_info: &ConnectionInfo,
) -> Result<Expression, CompileError> {
    let QueryDocument::Single(query) = query_doc else {
        return Err(CompileError::UnsupportedRequest);
    };

    let ctx = Context::new(connection_info, None);
    let (graph, _serializer) = QueryGraphBuilder::new(query_schema).build(query)?;
    let res: Result<Expression, TranslateError> = match connection_info.sql_family() {
        #[cfg(feature = "postgresql")]
        SqlFamily::Postgres => translate(graph, &SqlQueryBuilder::<visitor::Postgres<'_>>::new(ctx)),
        #[cfg(feature = "mysql")]
        SqlFamily::Mysql => translate(graph, &SqlQueryBuilder::<visitor::Mysql<'_>>::new(ctx)),
        #[cfg(feature = "sqlite")]
        SqlFamily::Sqlite => translate(graph, &SqlQueryBuilder::<visitor::Sqlite<'_>>::new(ctx)),
        #[cfg(feature = "mssql")]
        SqlFamily::Mssql => translate(graph, &SqlQueryBuilder::<visitor::Mssql<'_>>::new(ctx)),
    };

    res.map_err(CompileError::TranslateError)
}
