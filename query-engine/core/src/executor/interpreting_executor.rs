use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{Operation, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, Response, Responses};
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
    pub fn new(connector: C, primary_connector: &'static str, force_transactions: bool) -> Self {
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
    async fn execute(&self, operation: Operation, query_schema: QuerySchemaRef) -> crate::Result<Responses> {
        let conn = self.connector.get_connection().await?;

        // Parse, validate, and extract query graphs from query document.
        let (query, info) = QueryGraphBuilder::new(query_schema).build(operation)?;

        // Create pipelines for all separate queries
        let mut responses = Responses::with_capacity(1);
        let needs_transaction = self.force_transactions || query.needs_transaction();

        let result = if needs_transaction {
            let tx = conn.start_transaction().await?;

            let interpreter = QueryInterpreter::new(ConnectionLike::Transaction(tx.as_ref()));
            let result = QueryPipeline::new(query, interpreter, info).execute().await;

            if result.is_ok() {
                tx.commit().await?;
            } else {
                tx.rollback().await?;
            }

            result?
        } else {
            let interpreter = QueryInterpreter::new(ConnectionLike::Connection(conn.as_ref()));
            QueryPipeline::new(query, interpreter, info).execute().await?
        };

        match result {
            Response::Data(key, item) => responses.insert_data(key, item),
            Response::Error(error) => responses.insert_error(error),
        }

        Ok(responses)
    }

    fn primary_connector(&self) -> &'static str {
        self.primary_connector
    }
}
