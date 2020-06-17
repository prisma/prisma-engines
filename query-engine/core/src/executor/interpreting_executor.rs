use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{Operation, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, ResponseData};
use async_trait::async_trait;
use connector::{ConnectionLike, Connector};
use futures::{future, FutureExt};

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
    async fn execute_batch(
        &self,
        operations: Vec<Operation>,
        transactional: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        // if transactional {
        //     let queries = operations
        //         .into_iter()
        //         .map(|op| QueryGraphBuilder::new(query_schema.clone()).build(op))
        //         .collect::<std::result::Result<Vec<_>, _>>()?;

        //     let conn = self.connector.get_connection().await?;
        //     let tx = conn.start_transaction().await?;
        //     let mut results = Vec::with_capacity(queries.len());

        //     for (query, info) in queries {
        //         let interpreter = QueryInterpreter::new(ConnectionLike::Transaction(tx.as_ref()));
        //         let result = QueryPipeline::new(query, interpreter, info).execute().await;

        //         if !result.is_ok() {
        //             tx.rollback().await?;
        //         }

        //         results.push(result?.into());
        //     }

        //     Ok(results)
        // } else {
        //     let mut futures = Vec::with_capacity(operations.len());

        //     for operation in operations {
        //         futures.push(tokio::spawn(Self::execute(self, operation, query_schema.clone())));
        //     }

        //     // let responses = future::join_all(futures)
        //     //     .await
        //     //     .into_iter()
        //     //     .map(|res| res.expect("IO Error in tokio::spawn"))
        //     //     .collect();

        //     // Ok(responses.into_iter().map(|r: Response| r.into()).collect())
        //     todo!()
        // }

        todo!()
    }

    async fn execute(&self, operation: Operation, query_schema: QuerySchemaRef) -> crate::Result<ResponseData> {
        let conn = self.connector.get_connection().await?;

        // Parse, validate, and extract query graph from query document.
        let (query, serializer) = QueryGraphBuilder::new(query_schema).build(operation)?;
        let needs_transaction = self.force_transactions || query.needs_transaction();

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

    fn primary_connector(&self) -> &'static str {
        self.primary_connector
    }
}
