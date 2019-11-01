use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{
    CoreResult, IrSerializer, QueryDocument, QueryGraph, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, Response,
};
use connector::{Connector, AllOperations};
use futures::future::{BoxFuture, FutureExt};

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    connector: C,
    primary_connector: &'static str,
}

// Todo:
// - Partial execution semantics?
impl<C> InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    pub fn new(connector: C, primary_connector: &'static str) -> Self {
        InterpretingExecutor {
            connector,
            primary_connector,
        }
    }
}

impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    fn execute<'a>(
        &'a self,
        query_doc: QueryDocument,
        query_schema: QuerySchemaRef,
    ) -> BoxFuture<'a, CoreResult<Vec<Response>>> {
        let conn_fut = self.connector.get_connection();
        let fut = async move {
            let conn = conn_fut.await?;

            // Parse, validate, and extract query graphs from query document.
            let queries: Vec<(QueryGraph, IrSerializer)> = QueryGraphBuilder::new(query_schema).build(query_doc)?;

            // Create pipelines for all separate queries
            let mut results: Vec<Response> = vec![];

            for (query_graph, info) in queries {
                let tx = conn.start_transaction().await?;
                let interpreter = QueryInterpreter::new(tx.as_ref::<dyn AllOperations<'_>>());
                let result = QueryPipeline::new(query_graph, interpreter, info).execute().await;

                if result.is_ok() {
                    tx.commit().await?;
                } else {
                    tx.rollback().await?;
                }

                results.push(result?);
            }

            Ok(results)
        };

        fut.boxed()
    }

    fn primary_connector(&self) -> &'static str {
        self.primary_connector
    }
}
