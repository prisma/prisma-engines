pub mod expression;
pub mod translate;

use std::sync::Arc;

pub use expression::Expression;
use quaint::{
    prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily},
    visitor::Postgres,
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

pub fn compile(query_schema: &Arc<QuerySchema>, query_doc: QueryDocument) -> crate::Result<Expression> {
    let QueryDocument::Single(query) = query_doc else {
        return Err(CompileError::UnsupportedRequest.into());
    };
    let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
        SqlFamily::Postgres,
        "public".to_owned(),
        None,
    ));

    let ctx = Context::new(&connection_info, None);
    let builder = SqlQueryBuilder::<Postgres<'_>>::new(ctx);
    let (graph, _serializer) = QueryGraphBuilder::new(query_schema).build(query)?;
    Ok(translate(graph, &builder).map_err(CompileError::from)?)
}
