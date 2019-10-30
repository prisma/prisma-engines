use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{
    CoreResult, IrSerializer, QueryDocument, QueryGraph, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, Response,
};
use connector::{Connector, Result as ConnectorResult, TransactionLike};
use futures::future::{BoxFuture, FutureExt};

/// Central query executor and main entry point into the query core.
/// Interprets the full query tree to return a result.
pub struct InterpretingExecutor<C> {
    connector: C,
    primary_connector: &'static str,
}

// Todo:
// - Partial execution semantics?
// - ReadQueryResult + write query results should probably just be QueryResult
impl<C> InterpretingExecutor<C>
where
    C: Connector + Send + Sync + 'static,
{
    pub fn new(connector: C, primary_connector: &'static str) -> Self {
        InterpretingExecutor {
            connector,
            primary_connector,
        }
    }

    pub fn with_interpreter<'a, F>(&self, f: F) -> ConnectorResult<Response>
    where
        F: FnOnce(QueryInterpreter) -> ConnectorResult<Response>,
    {
        let res = self.connector.with_transaction(|tx: &mut dyn TransactionLike| {
            let interpreter = QueryInterpreter::new(tx);

            f(interpreter).map_err(|err| err.into())
        });

        res
    }
}

impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync + 'static,
{
    fn execute(&self, query_doc: QueryDocument, query_schema: QuerySchemaRef) -> BoxFuture<CoreResult<Vec<Response>>> {
        let fut = async move {
            // Parse, validate, and extract query graphs from query document.
            let queries: Vec<(QueryGraph, IrSerializer)> = QueryGraphBuilder::new(query_schema).build(query_doc)?;

            // Create pipelines for all separate queries
            Ok(queries
                .into_iter()
                .map(|(query_graph, info)| {
                    self.with_interpreter(|interpreter| {
                        QueryPipeline::new(query_graph, interpreter, info)
                            .execute()
                            .map_err(|err| err.into())
                    })
                })
                .collect::<ConnectorResult<Vec<Response>>>()?)
        };

        fut.boxed()
    }

    fn primary_connector(&self) -> &'static str {
        self.primary_connector
    }
}
