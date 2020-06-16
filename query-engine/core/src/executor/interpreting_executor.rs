use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{Operation, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, Response, Responses};
use async_trait::async_trait;
use connector::{ConnectionLike, Connector};

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    connector: C,
    primary_connector: &'static str,

    /// Flag that forces individual operations to run in a transaction.
    /// Does _not_ force batches to use transactions.
    force_transactions: bool,
}

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

// We have to distinguish between per-query needs and per-batch needs for transaction.
// If a batch is not transactional:
// - individual writes still need to run in a per-write transaction basis.
// - We fan out the queries to as many connections as possible
#[async_trait]
impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    // Q: Transactional for a batch can mean 2 things:
    // - Abort if one op fails, but all can be done in parallel.
    // - Abort if one op fails, all operations are done in sequence. < This is the common understanding. Validate with product.
    async fn execute_batch(
        &self,
        operations: Vec<Operation>,
        transactional: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<Vec<Responses>> {
        if transactional {
            todo!()
        } else {
            todo!()
        }
    }

    async fn execute(&self, operation: Operation, query_schema: QuerySchemaRef) -> crate::Result<Responses> {
        let conn = self.connector.get_connection().await?;

        // Parse, validate, and extract query graph from query document.
        let (query, info) = QueryGraphBuilder::new(query_schema).build(operation)?;

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
