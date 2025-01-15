pub mod expression;
pub mod translate;

use std::sync::Arc;

pub use expression::Expression;
use schema::QuerySchema;
use thiserror::Error;
use quaint::connector::ConnectionInfo;
pub use translate::{translate, TranslateError};

use crate::{QueryDocument, QueryGraphBuilder};

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("only a single query can be compiled at a time")]
    UnsupportedRequest,

    #[error("{0}")]
    TranslateError(#[from] TranslateError),
}

pub fn compile(query_schema: &Arc<QuerySchema>, query_doc: QueryDocument, connection_info: &ConnectionInfo) -> crate::Result<Expression> {
    let QueryDocument::Single(query) = query_doc else {
        return Err(CompileError::UnsupportedRequest.into());
    };

    let (graph, _serializer) = QueryGraphBuilder::new(query_schema).build(query)?;
    Ok(translate(graph, connection_info).map_err(CompileError::from)?)
}
