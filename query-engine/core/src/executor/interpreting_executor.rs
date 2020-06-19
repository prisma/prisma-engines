use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{Operation, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, ResponseData};
use async_trait::async_trait;
use connector::{Connection, ConnectionLike, Connector};
use futures::future;

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

    /// Async wrapper for executing an individual operation to allow code sharing with `execute_batch`.
    async fn execute_single_operation(
        operation: Operation,
        conn: Box<dyn Connection>,
        force_transactions: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<ResponseData> {
        // Parse, validate, and extract query graph from query document.
        let (query, serializer) = QueryGraphBuilder::new(query_schema).build(operation)?;
        let needs_transaction = force_transactions || query.needs_transaction();

        if needs_transaction {
            let tx = conn.start_transaction().await?;
            let interpreter = QueryInterpreter::new(ConnectionLike::Transaction(tx.as_ref()));
            let result = QueryPipeline::new(query, interpreter, serializer).execute().await;

            if result.is_ok() {
                tx.commit().await?;
            } else {
                tx.rollback().await?;
            }

            result
        } else {
            let interpreter = QueryInterpreter::new(ConnectionLike::Connection(conn.as_ref()));
            QueryPipeline::new(query, interpreter, serializer).execute().await
        }
    }
}

#[async_trait]
impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync + 'static,
{
    /// Executes a batch of operations.
    ///
    /// If the batch is to be executed transactionally:
    /// All operations are evaluated in sequence and the entire batch is rolled back if one operation fails,
    /// returning the error.
    ///
    /// If the batch is not transactional:
    /// All operations are fanned out onto as many connections as possible and executed independently.
    /// A failing operation does not fail the batch, instead, an error is returned alongside other responses.
    /// Note that individual operations executed in non-transactional mode can still be transactions in themselves
    /// if the query (e.g. a write op) requires it.
    async fn execute_batch(
        &self,
        operations: Vec<Operation>,
        transactional: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        if transactional {
            let queries = operations
                .into_iter()
                .map(|op| QueryGraphBuilder::new(query_schema.clone()).build(op))
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let conn = self.connector.get_connection().await?;
            let tx = conn.start_transaction().await?;
            let mut results = Vec::with_capacity(queries.len());

            for (query, info) in queries {
                let interpreter = QueryInterpreter::new(ConnectionLike::Transaction(tx.as_ref()));
                let result = QueryPipeline::new(query, interpreter, info).execute().await;

                if !result.is_ok() {
                    tx.rollback().await?;
                }

                results.push(Ok(result?));
            }

            Ok(results)
        } else {
            let mut futures = Vec::with_capacity(operations.len());

            for operation in operations {
                let conn = self.connector.get_connection().await?;
                futures.push(tokio::spawn(Self::execute_single_operation(
                    operation,
                    conn,
                    self.force_transactions,
                    query_schema.clone(),
                )));
            }

            let responses: Vec<_> = future::join_all(futures)
                .await
                .into_iter()
                .map(|res| res.expect("IO Error in tokio::spawn"))
                .collect();

            Ok(responses)
        }
    }

    /// Executes a single operation. Execution will be inside of a transaction or not depending on the needs of the query.
    async fn execute(&self, operation: Operation, query_schema: QuerySchemaRef) -> crate::Result<ResponseData> {
        let conn = self.connector.get_connection().await?;
        Self::execute_single_operation(operation, conn, self.force_transactions, query_schema.clone()).await
    }

    fn primary_connector(&self) -> &'static str {
        self.primary_connector
    }
}
