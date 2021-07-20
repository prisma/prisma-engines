use std::{marker::PhantomPinned, ops::DerefMut, pin::Pin};

use super::{pipeline::QueryPipeline, QueryExecutor};
use crate::{Operation, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, ResponseData, TransactionManager, TxId};
use async_trait::async_trait;
use connector::{Connection, Connector, Transaction};
use dashmap::DashMap;
use futures::future;

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    /// The loaded connector
    connector: C,

    tx_cache: TransactionCache,

    /// Flag that forces individual operations to run in a transaction.
    /// Does _not_ force batches to use transactions.
    force_transactions: bool,
}

#[derive(Default)]
struct TransactionCache {
    pub cache: DashMap<TxId, Pin<Box<CachedTx>>>,
}

struct CachedTx {
    pub conn: Box<dyn Connection>,
    pub tx: Option<Box<dyn Transaction + 'static>>,
    pub status: TxStatus,
}

impl CachedTx {
    pub async fn new(conn: Box<dyn Connection>) -> crate::Result<Pin<Box<Self>>> {
        let c_tx = CachedTx {
            conn,
            tx: None,
            status: TxStatus::Open,
        };

        let mut boxed = Box::pin(c_tx);
        unsafe {
            let transaction: Box<dyn Transaction + '_> =
                boxed.as_mut().get_unchecked_mut().conn.start_transaction().await?;

            let transaction: Box<dyn Transaction + 'static> = std::mem::transmute(transaction);
            boxed.tx = Some(transaction);
        }

        Ok(boxed)
    }
}

enum TxStatus {
    Open,
    Committed,
    RolledBack,
    Expired,
}

impl<C> InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    pub fn new(connector: C, force_transactions: bool) -> Self {
        InterpretingExecutor {
            connector,
            tx_cache: TransactionCache::default(),
            force_transactions,
        }
    }

    /// Async wrapper for executing an individual operation to allow code sharing with `execute_batch`.
    #[tracing::instrument(skip(operation, conn, force_transactions, query_schema))]
    async fn execute_single_operation(
        operation: Operation,
        mut conn: Box<dyn Connection>,
        force_transactions: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<ResponseData> {
        // Parse, validate, and extract query graph from query document.
        let (query_graph, serializer) = QueryGraphBuilder::new(query_schema).build(operation)?;
        let is_transactional = force_transactions || query_graph.needs_transaction();

        if is_transactional {
            let mut tx = conn.start_transaction().await?;
            let interpreter = QueryInterpreter::new(tx.as_connection_like());
            let result = QueryPipeline::new(query_graph, interpreter, serializer).execute().await;

            if result.is_ok() {
                tx.commit().await?;
            } else {
                tx.rollback().await?;
            }

            result
        } else {
            let interpreter = QueryInterpreter::new(conn.as_connection_like());

            QueryPipeline::new(query_graph, interpreter, serializer).execute().await
        }
    }
}

#[async_trait]
impl<C> QueryExecutor for InterpretingExecutor<C>
where
    C: Connector + Send + Sync + 'static,
{
    /// Executes a single operation. Execution will be inside of a transaction or not depending on the needs of the query.
    async fn execute(
        &self,
        tx_id: Option<TxId>,
        operation: Operation,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<ResponseData> {
        let conn = self.connector.get_connection().await?;

        Self::execute_single_operation(operation, conn, self.force_transactions, query_schema.clone()).await
    }

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
    #[tracing::instrument(skip(self, operations, query_schema))]
    async fn execute_all(
        &self,
        tx_id: Option<TxId>,
        operations: Vec<Operation>,
        transactional: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        if transactional {
            let queries = operations
                .into_iter()
                .map(|op| QueryGraphBuilder::new(query_schema.clone()).build(op))
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let mut conn = self.connector.get_connection().await?;
            let mut tx = conn.start_transaction().await?;
            let mut results = Vec::with_capacity(queries.len());

            for (query, info) in queries {
                let interpreter = QueryInterpreter::new(tx.as_connection_like());
                let result = QueryPipeline::new(query, interpreter, info).execute().await;

                if result.is_err() {
                    tx.rollback().await?;
                }

                results.push(Ok(result?));
            }

            tx.commit().await?;
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

    fn primary_connector(&self) -> &(dyn Connector + Send + Sync) {
        &self.connector
    }
}

#[async_trait]
impl<C> TransactionManager for InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    async fn start_tx(&self, max_acquisition_secs: u32, valid_for_secs: u32) -> crate::Result<TxId> {
        let id = TxId::new();
        let mut conn = self.connector.get_connection().await?;

        let c_tx = CachedTx::new(conn).await?;
        self.tx_cache.cache.insert(id.clone(), c_tx);

        Ok(id)
    }

    async fn commit_tx(&self, tx_id: TxId) -> crate::Result<TxId> {
        todo!()
    }

    async fn rollback_tx(&self, tx_id: TxId) -> crate::Result<TxId> {
        todo!()
    }
}
