use super::execute_operation::{execute_many_operations, execute_many_self_contained, execute_single_self_contained};
use super::request_context;
use crate::ItxManager;
use crate::{
    protocol::EngineProtocol, BatchDocumentTransaction, CoreError, Operation, QueryExecutor, ResponseData,
    TransactionError, TransactionManager, TransactionOptions, TxId,
};

use async_trait::async_trait;
use connector::Connector;
use schema::QuerySchemaRef;
use telemetry::helpers::TraceParent;
use tokio::time::Duration;
use tracing_futures::Instrument;

/// Central query executor and main entry point into the query core.
pub struct InterpretingExecutor<C> {
    /// The loaded connector
    connector: C,

    itx_manager: ItxManager,

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
            itx_manager: ItxManager::new(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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
        traceparent: Option<TraceParent>,
        engine_protocol: EngineProtocol,
    ) -> crate::Result<ResponseData> {
        request_context::with_request_context(engine_protocol, async move {
            if let Some(tx_id) = tx_id {
                self.itx_manager.execute(&tx_id, operation, traceparent).await
            } else {
                execute_single_self_contained(
                    &self.connector,
                    query_schema,
                    operation,
                    traceparent,
                    self.force_transactions,
                )
                .await
            }
        })
        .await
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
        traceparent: Option<TraceParent>,
        engine_protocol: EngineProtocol,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        request_context::with_request_context(engine_protocol, async move {
            if let Some(tx_id) = tx_id {
                let batch_isolation_level = transaction.and_then(|t| t.isolation_level());
                if batch_isolation_level.is_some() {
                    return Err(CoreError::UnsupportedFeatureError(
                        "Can not set batch isolation level within interactive transaction".into(),
                    ));
                }
                self.itx_manager.batch_execute(&tx_id, operations, traceparent).await
            } else if let Some(transaction) = transaction {
                let conn_span = info_span!(
                    "prisma:engine:connection",
                    user_facing = true,
                    "db.type" = self.connector.name(),
                );
                let mut conn = self.connector.get_connection().instrument(conn_span).await?;
                let mut tx = conn.start_transaction(transaction.isolation_level()).await?;

                let results =
                    execute_many_operations(query_schema, tx.as_connection_like(), &operations, traceparent).await;

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
                    traceparent,
                    self.force_transactions,
                    engine_protocol,
                )
                .await
            }
        })
        .await
    }

    fn primary_connector(&self) -> &(dyn Connector + Send + Sync) {
        &self.connector
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> TransactionManager for InterpretingExecutor<C>
where
    C: Connector + Send + Sync,
{
    async fn start_tx(
        &self,
        query_schema: QuerySchemaRef,
        engine_protocol: EngineProtocol,
        tx_opts: TransactionOptions,
    ) -> crate::Result<TxId> {
        super::with_request_context(engine_protocol, async move {
            let isolation_level = tx_opts.isolation_level;
            let valid_for_millis = tx_opts.valid_for_millis;
            let id = tx_opts.new_tx_id.unwrap_or_default();

            let conn_span = info_span!(
                "prisma:engine:connection",
                user_facing = true,
                "db.type" = self.connector.name()
            );
            let conn = crosstarget_utils::time::timeout(
                Duration::from_millis(tx_opts.max_acquisition_millis),
                self.connector.get_connection(),
            )
            .instrument(conn_span)
            .await;

            let conn = conn.map_err(|_| TransactionError::AcquisitionTimeout)??;

            self.itx_manager
                .create_tx(
                    query_schema.clone(),
                    id.clone(),
                    conn,
                    isolation_level,
                    Duration::from_millis(valid_for_millis),
                )
                .await?;

            Ok(id)
        })
        .await
    }

    async fn commit_tx(&self, tx_id: TxId) -> crate::Result<()> {
        self.itx_manager.commit_tx(&tx_id).await
    }

    async fn rollback_tx(&self, tx_id: TxId) -> crate::Result<()> {
        self.itx_manager.rollback_tx(&tx_id).await
    }
}
