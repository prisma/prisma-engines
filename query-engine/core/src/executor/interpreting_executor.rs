use super::execute_operation::{execute_many_operations, execute_many_self_contained, execute_single_self_contained};
use crate::{
    BatchDocumentTransaction, CoreError, OpenTx, Operation, QueryExecutor, ResponseData, TransactionActorManager,
    TransactionError, TransactionManager, TransactionOptions, TxId,
};

use async_trait::async_trait;
use connector::Connector;
use schema::QuerySchemaRef;
use tokio::time::{self, Duration};
use tracing_futures::Instrument;

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
            super::with_request_now(async move {
                execute_single_self_contained(
                    &self.connector,
                    query_schema,
                    operation,
                    trace_id,
                    self.force_transactions,
                )
                .await
            })
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
    async fn execute_all(
        &self,
        tx_id: Option<TxId>,
        operations: Vec<Operation>,
        transaction: Option<BatchDocumentTransaction>,
        query_schema: QuerySchemaRef,
        trace_id: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        if let Some(tx_id) = tx_id {
            let batch_isolation_level = transaction.and_then(|t| t.isolation_level());
            if batch_isolation_level.is_some() {
                return Err(CoreError::UnsupportedFeatureError(
                    "Can not set batch isolation level within interactive transaction".into(),
                ));
            }
            self.itx_manager.batch_execute(&tx_id, operations, trace_id).await
        } else if let Some(transaction) = transaction {
            let conn_span = info_span!(
                "prisma:engine:connection",
                user_facing = true,
                "db.type" = self.connector.name(),
            );
            let mut conn = self.connector.get_connection().instrument(conn_span).await?;
            let mut tx = conn.start_transaction(transaction.isolation_level()).await?;

            let results = super::with_request_now(execute_many_operations(
                query_schema,
                tx.as_connection_like(),
                &operations,
                trace_id,
            ))
            .await;

            if results.is_err() {
                tx.rollback().await?;
            } else {
                tx.commit().await?;
            }

            results
        } else {
            super::with_request_now(async move {
                execute_many_self_contained(
                    &self.connector,
                    query_schema,
                    &operations,
                    trace_id,
                    self.force_transactions,
                )
                .await
            })
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
    async fn start_tx(&self, query_schema: QuerySchemaRef, tx_opts: &TransactionOptions) -> crate::Result<TxId> {
        super::with_request_now(async move {
            let id = if let Some(id_str) = tx_opts.predefined_id.clone() {
                id_str.into()
            } else {
                TxId::default()
            };
            trace!("[{}] Starting...", id);
            let conn_span = info_span!(
                "prisma:engine:connection",
                user_facing = true,
                "db.type" = self.connector.name()
            );
            let conn = time::timeout(
                Duration::from_millis(tx_opts.max_acquisition_millis),
                self.connector.get_connection(),
            )
            .instrument(conn_span)
            .await;

            let conn = conn.map_err(|_| TransactionError::AcquisitionTimeout)??;
            let c_tx = OpenTx::start(conn, tx_opts.isolation_level.clone()).await?;

            self.itx_manager
                .create_tx(
                    query_schema.clone(),
                    id.clone(),
                    c_tx,
                    Duration::from_millis(tx_opts.valid_for_millis),
                )
                .await;

            debug!("[{}] Started.", id);
            Ok(id)
        })
        .await
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
