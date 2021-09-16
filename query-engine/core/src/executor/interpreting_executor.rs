use super::{
    interactive_tx::{CachedTx, TransactionCache, TxId},
    pipeline::QueryPipeline,
    QueryExecutor,
};
use crate::{
    IrSerializer, OpenTx, Operation, QueryGraph, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, ResponseData,
    TransactionError, TransactionManager,
};
use async_trait::async_trait;
use connector::{error::ErrorKind, Connection, ConnectionLike, Connector};
use futures::{future, Future};
use tokio::time;

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    /// The loaded connector
    connector: C,

    tx_cache: TransactionCache,

    /// Flag that forces individual operations to run in a transaction.
    /// Does _not_ force batches to use transactions.
    force_transactions: bool,
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

    /// Execute the operation as a self-contained operation, if necessary wrapped in a transaction.
    #[tracing::instrument(skip(conn, graph, serializer, force_transactions))]
    async fn execute_self_contained(
        mut conn: Box<dyn Connection>,
        graph: QueryGraph,
        serializer: IrSerializer,
        force_transactions: bool,
    ) -> crate::Result<ResponseData> {
        if force_transactions || graph.needs_transaction() {
            let mut tx = conn.start_transaction().await?;
            let result = Self::execute_on(tx.as_connection_like(), graph, serializer).await;

            if result.is_ok() {
                tx.commit().await?;
            } else {
                tx.rollback().await?;
            }

            result
        } else {
            Self::execute_on(conn.as_connection_like(), graph, serializer).await
        }
    }

    /// Simplest execution on anything that's a ConnectionLike. Caller decides handling of connections and transactions.
    #[tracing::instrument(skip(conn, graph, serializer))]
    async fn execute_on(
        conn: &mut dyn ConnectionLike,
        graph: QueryGraph,
        serializer: IrSerializer,
    ) -> crate::Result<ResponseData> {
        let interpreter = QueryInterpreter::new(conn);
        let result = QueryPipeline::new(graph, interpreter, serializer).execute().await;

        result
    }

    async fn finalize_tx<F>(&self, tx_id: TxId, final_state: CachedTx, finalizer: F) -> crate::Result<()>
    where
        F: Fn(&mut OpenTx) -> Box<dyn Future<Output = connector::Result<()>> + Unpin + Send + '_>,
    {
        // The references need to be dropped before finalization,
        // or else the DashMap deadlocks with the finalization cleanup task.
        let final_state = {
            let mut tx = self.tx_cache.get_or_err(&tx_id)?;
            let otx = tx.as_open()?;

            // Some connectors hard-abort transactions after an error
            // and refuse to execute any subsequent operation. Handle cleanup for those cases.
            let state = if let Err(err) = finalizer(otx).await {
                if let ErrorKind::TransactionAborted { message } = err.kind {
                    debug!("[{}] Aborted with {}.", tx_id, message);
                    CachedTx::Aborted
                } else {
                    return Err(err.into());
                }
            } else {
                final_state
            };

            otx.cancel_expiration_timer();
            state
        };

        self.tx_cache.finalize_tx(tx_id, final_state);
        Ok(())
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
        // Parse, validate, and extract query graph from query document.
        let (query_graph, serializer) = QueryGraphBuilder::new(query_schema).build(operation)?;

        // If a Tx id is provided, execute on that one. Else execute normally as a single operation.
        if let Some(tx_id) = tx_id {
            let mut c_tx = self.tx_cache.get_or_err(&tx_id)?;
            let otx = c_tx.as_open()?;

            Self::execute_on(otx.tx.as_connection_like(), query_graph, serializer).await
        } else {
            let conn = self.connector.get_connection().await?;
            Self::execute_self_contained(conn, query_graph, serializer, self.force_transactions).await
        }
    }

    /// Executes a batch of operations.
    ///
    /// If the batch is to be executed transactionally:
    /// - TX ID is provided: All operations are evaluated in sequence, stop execution on error and return the error.
    /// - No TX ID && if `transactional: true`: All operations are evaluated in sequence and the entire batch is rolled back if
    ///   one operation fails, returning the error.
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
        if let Some(tx_id) = tx_id {
            let queries = operations
                .into_iter()
                .map(|op| QueryGraphBuilder::new(query_schema.clone()).build(op))
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let mut c_tx = self.tx_cache.get_or_err(&tx_id)?;
            let otx = c_tx.as_open()?;
            let mut results = Vec::with_capacity(queries.len());

            let tx = otx.as_connection_like();

            for (graph, serializer) in queries {
                let result = Self::execute_on(tx, graph, serializer).await?;
                results.push(Ok(result));
            }

            Ok(results)
        } else if transactional {
            let queries = operations
                .into_iter()
                .map(|op| QueryGraphBuilder::new(query_schema.clone()).build(op))
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let mut conn = self.connector.get_connection().await?;
            let mut tx = conn.start_transaction().await?;
            let mut results = Vec::with_capacity(queries.len());

            for (graph, serializer) in queries {
                let result = Self::execute_on(tx.as_connection_like(), graph, serializer).await;

                if result.is_err() {
                    tx.rollback().await?;
                }

                results.push(Ok(result?));
            }

            tx.commit().await?;
            Ok(results)
        } else {
            let mut futures = Vec::with_capacity(operations.len());

            for op in operations {
                match QueryGraphBuilder::new(query_schema.clone()).build(op) {
                    Ok((graph, serializer)) => {
                        let conn = self.connector.get_connection().await?;

                        futures.push(tokio::spawn(Self::execute_self_contained(
                            conn,
                            graph,
                            serializer,
                            self.force_transactions,
                        )));
                    }

                    // This looks unnecessary, but is the simplest way to preserve ordering of results for the batch.
                    Err(err) => futures.push(tokio::spawn(async move { Err(err.into()) })),
                }
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
    async fn start_tx(&self, max_acquisition_millis: u64, valid_for_millis: u64) -> crate::Result<TxId> {
        let id = TxId::default();
        debug!("[{}] Starting...", id);

        let conn = time::timeout(
            time::Duration::from_millis(max_acquisition_millis),
            self.connector.get_connection(),
        )
        .await;

        let conn = conn.map_err(|_| TransactionError::AcquisitionTimeout)??;
        let c_tx = OpenTx::start(conn).await?;

        self.tx_cache.insert(id.clone(), c_tx, valid_for_millis).await;

        debug!("[{}] Started.", id);
        Ok(id)
    }

    async fn commit_tx(&self, tx_id: TxId) -> crate::Result<()> {
        debug!("[{}] Committing.", tx_id);
        self.finalize_tx(tx_id, CachedTx::Committed, |otx| Box::new(otx.tx.commit()))
            .await
    }

    async fn rollback_tx(&self, tx_id: TxId) -> crate::Result<()> {
        debug!("[{}] Rolling back.", tx_id);
        self.finalize_tx(tx_id, CachedTx::RolledBack, |otx| Box::new(otx.tx.rollback()))
            .await
    }
}
