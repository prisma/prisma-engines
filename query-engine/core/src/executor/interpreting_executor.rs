use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{
    CoreResult, IrSerializer, QueryDocument, QueryGraph, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, Response, Responses,
};
use async_trait::async_trait;
use connector::{ConnectionLike, Connector};

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    connector: C,
    primary_connector: &'static str,
    force_transactions: bool,
}

// Todo:
// - Partial execution semantics?
impl<C> InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    pub fn new(
        connector: C,
        primary_connector: &'static str,
        force_transactions: bool,
    ) -> Self
    {
        InterpretingExecutor {
            connector,
            primary_connector,
            force_transactions,
        }
    }
}

#[async_trait]
impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    async fn execute(&self, query_doc: QueryDocument, query_schema: QuerySchemaRef) -> CoreResult<Responses> {
        let conn = self.connector.get_connection().await?;

        // Parse, validate, and extract query graphs from query document.
        let queries: Vec<(QueryGraph, IrSerializer)> = QueryGraphBuilder::new(query_schema).build(query_doc)?;

        // Create pipelines for all separate queries
        let mut responses = Responses::with_capacity(queries.len());

        for (query_graph, info) in queries {
            let needs_transaction = self.force_transactions || query_graph.needs_transaction();

            let result = if needs_transaction {
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

            match result {
                Response::Data(key, item) => responses.insert_data(key, item),
                Response::Error(error) => responses.insert_error(error),
            }
        }

        Ok(responses)
    }

    fn primary_connector(&self) -> &'static str {
        self.primary_connector
    }
}
