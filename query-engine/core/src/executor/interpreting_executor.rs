use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{
    CoreResult, IrSerializer, QueryDocument, QueryGraph, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, Response,
};
use connector::{Connector, ConnectionLike};
use async_trait::async_trait;

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

#[async_trait]
impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    async fn execute(
        &self,
        query_doc: QueryDocument,
        query_schema: QuerySchemaRef,
    ) -> CoreResult<Vec<Response>> {
        let conn = self.connector.get_connection().await?;

        // Parse, validate, and extract query graphs from query document.
        let queries: Vec<(QueryGraph, IrSerializer)> = QueryGraphBuilder::new(query_schema).build(query_doc)?;

        // Create pipelines for all separate queries
        let mut results: Vec<Response> = vec![];

        for (query_graph, info) in queries {
            let result = if query_graph.needs_transaction() {
                let tx = conn.start_transaction().await?;

                let interpreter = QueryInterpreter::new(ConnectionLike::Transaction(tx.as_ref()));
                let result = QueryPipeline::new(query_graph, interpreter, info).execute().await;

                if result.is_ok() {
                    tx.commit().await?;
                } else {
                    tx.rollback().await?;
                }

                result?
            } else {
                let interpreter = QueryInterpreter::new(ConnectionLike::Connection(conn.as_ref()));
                QueryPipeline::new(query_graph, interpreter, info).execute().await?
            };

            results.push(result);
        }

        Ok(results)
    }

    fn primary_connector(&self) -> &'static str {
        self.primary_connector
    }
}
