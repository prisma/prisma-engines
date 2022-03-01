use super::execute_operation::{execute_many_operations, execute_many_self_contained, execute_single_self_contained};
use crate::{
    OpenTx, Operation, QueryExecutor, QuerySchemaRef, ResponseData, TransactionActorManager, TransactionError,
    TransactionManager, TxId,
};

use async_trait::async_trait;
use connector::Connector;
use tokio::time::{self, Duration};

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    /// The loaded connector
    connector: C,

    itx_manager: TransactionActorManager,

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
            itx_manager: TransactionActorManager::new(),
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
        trace_id: Option<String>,
    ) -> crate::Result<ResponseData> {
        // If a Tx id is provided, execute on that one. Else execute normally as a single operation.
        if let Some(tx_id) = tx_id {
            self.itx_manager.execute(&tx_id, operation, trace_id).await
        } else {
            execute_single_self_contained(
                &self.connector,
                query_schema,
                operation,
                trace_id,
                self.force_transactions,
            )
            .await
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
            let mut conn = self.connector.get_connection().await?;
            let mut tx = conn.start_transaction().await?;

            let results = execute_many_operations(query_schema, tx.as_connection_like(), &operations, trace_id).await;

            if results.is_err() {
                tx.rollback().await?;
            } else {
                tx.commit().await?;
            }

            results
        } else {
            execute_many_self_contained(
                &self.connector,
                query_schema,
                &operations,
                trace_id,
                self.force_transactions,
            )
            .await
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
        trace!("[{}] Starting...", id);
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
        trace!("[{}] Committing.", tx_id);
        self.itx_manager.commit_tx(&tx_id).await
    }

    async fn rollback_tx(&self, tx_id: TxId) -> crate::Result<()> {
        trace!("[{}] Rolling back.", tx_id);
        self.itx_manager.rollback_tx(&tx_id).await
    }
}
