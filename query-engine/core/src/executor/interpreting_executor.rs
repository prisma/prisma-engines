use super::{
    interactive_tx::{CachedTx, TxId},
    pipeline::QueryPipeline,
    QueryExecutor,
};
use crate::{
    IrSerializer, OpenTx, Operation, QueryGraph, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, ResponseData,
    TransactionError, TransactionManager, TransactionProcessManager,
};
use async_trait::async_trait;
use connector::{Connection, ConnectionLike, Connector};
use futures::future;
use tokio::time::{self, Duration};

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    /// The loaded connector
    connector: C,

    itx_manager: TransactionProcessManager,

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
            force_transactions,
            itx_manager: TransactionProcessManager::new(),
        }
    }

    /// Execute the operation as a self-contained operation, if necessary wrapped in a transaction.
    #[tracing::instrument(skip(conn, graph, serializer, force_transactions))]
    async fn execute_self_contained(
        mut conn: Box<dyn Connection>,
        graph: QueryGraph,
        serializer: IrSerializer,
        force_transactions: bool,
        trace_id: Option<String>,
    ) -> crate::Result<ResponseData> {
        if force_transactions || graph.needs_transaction() {
            let mut tx = conn.start_transaction().await?;
            let result = Self::execute_on(tx.as_connection_like(), graph, serializer, trace_id).await;

            if result.is_ok() {
                tx.commit().await?;
            } else {
                tx.rollback().await?;
            }

            result
        } else {
            Self::execute_on(conn.as_connection_like(), graph, serializer, trace_id).await
        }
    }

    /// Simplest execution on anything that's a ConnectionLike. Caller decides handling of connections and transactions.
    #[tracing::instrument(skip(conn, graph, serializer))]
    async fn execute_on(
        conn: &mut dyn ConnectionLike,
        graph: QueryGraph,
        serializer: IrSerializer,
        trace_id: Option<String>,
    ) -> crate::Result<ResponseData> {
        let interpreter = QueryInterpreter::new(conn);
        let result = QueryPipeline::new(graph, interpreter, serializer)
            .execute(trace_id)
            .await;

        result
    }

    async fn finalize_tx(&self, tx_id: TxId, final_state: CachedTx) -> crate::Result<()> {
        match final_state {
            CachedTx::Committed => self.itx_manager.commit_tx(&tx_id).await?,
            CachedTx::RolledBack => self.itx_manager.rollback_tx(&tx_id).await?,
            _ => unreachable!(),
        };
        debug!("[{tx_id}] FINALIZE DONE {final_state}");

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
        trace_id: Option<String>,
    ) -> crate::Result<ResponseData> {
        // If a Tx id is provided, execute on that one. Else execute normally as a single operation.
        if let Some(tx_id) = tx_id {
            self.itx_manager.execute(&tx_id, operation, trace_id).await
        } else {
            let (query_graph, serializer) = QueryGraphBuilder::new(query_schema).build(operation)?;
            let conn = self.connector.get_connection().await?;
            Self::execute_self_contained(conn, query_graph, serializer, self.force_transactions, trace_id).await
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
        trace_id: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        if let Some(tx_id) = tx_id {
            self.itx_manager.batch_execute(&tx_id, operations, trace_id).await
        } else if transactional {
            let queries = operations
                .into_iter()
                .map(|op| QueryGraphBuilder::new(query_schema.clone()).build(op))
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let mut conn = self.connector.get_connection().await?;
            let mut tx = conn.start_transaction().await?;
            let mut results = Vec::with_capacity(queries.len());

            for (graph, serializer) in queries {
                let result = Self::execute_on(tx.as_connection_like(), graph, serializer, trace_id.clone()).await;

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
                            trace_id.clone(),
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
    async fn start_tx(
        &self,
        query_schema: QuerySchemaRef,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
    ) -> crate::Result<TxId> {
        let id = TxId::default();
        debug!("[{}] Starting...", id);
        let conn = time::timeout(
            Duration::from_millis(max_acquisition_millis),
            self.connector.get_connection(),
        )
        .await;

        let conn = conn.map_err(|_| TransactionError::AcquisitionTimeout)??;
        let c_tx = OpenTx::start(conn).await?;

        self.itx_manager
            .create_tx(
                query_schema.clone(),
                id.clone(),
                c_tx,
                Duration::from_millis(valid_for_millis),
            )
            .await;

        debug!("[{}] Started.", id);
        Ok(id)
    }

    async fn commit_tx(&self, tx_id: TxId) -> crate::Result<()> {
        debug!("[{}] Committing.", tx_id);
        self.finalize_tx(tx_id, CachedTx::Committed).await
    }

    async fn rollback_tx(&self, tx_id: TxId) -> crate::Result<()> {
        debug!("[{}] Rolling back.", tx_id);
        self.finalize_tx(tx_id, CachedTx::RolledBack).await
    }
}
