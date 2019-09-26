use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{
    CoreResult, IrSerializer, QueryGraphBuilder, QueryDocument, QueryGraph, QueryInterpreter, QuerySchemaRef, Response,
};
use connector::{Connector, TransactionLike};

/// Central query executor and main entry point into the query core.
/// Interprets the full query tree to return a result.
pub struct InterpretingExecutor<C> {
    connector: C,
}

// Todo:
// - Partial execution semantics?
// - ReadQueryResult + write query results should probably just be QueryResult
impl<C> InterpretingExecutor<C>
where
    C: Connector + Send + Sync + 'static,
{
    pub fn new(connector: C) -> Self {
        InterpretingExecutor { connector }
    }

    pub fn with_interpreter<'a, F>(&self, f: F) -> CoreResult<Response>
    where
        F: FnOnce(QueryInterpreter) -> CoreResult<Response>,
    {
        let res = self.connector.with_transaction(|tx: &mut dyn TransactionLike| {
            let interpreter = QueryInterpreter { tx };

            Ok(f(interpreter))
        })?;

        res
    }
}

impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync + 'static,
{
    fn execute(&self, query_doc: QueryDocument, query_schema: QuerySchemaRef) -> CoreResult<Vec<Response>> {
        // Parse, validate, and extract query graphs from query document.
        let queries: Vec<(QueryGraph, IrSerializer)> = QueryGraphBuilder::new(query_schema).build(query_doc)?;

        // Create pipelines for all separate queries
        queries
            .into_iter()
            .map(|(query_graph, info)| {
                self.with_interpreter(|interpreter| QueryPipeline::new(query_graph, interpreter, info).execute())
            })
            .collect()
    }
}
