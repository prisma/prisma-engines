use crate::{protocol::EngineProtocol, ClosedTx, CoreError, InteractiveTransaction, Operation, ResponseData};
use connector::Connection;
use lru::LruCache;
use once_cell::sync::Lazy;
use schema::QuerySchemaRef;
use std::collections::HashMap;
use tokio::{sync::RwLock, time::Duration};

use super::{TransactionError, TxId};

pub static CLOSED_TX_CACHE_SIZE: Lazy<usize> = Lazy::new(|| match std::env::var("CLOSED_TX_CACHE_SIZE") {
    Ok(size) => size.parse().unwrap_or(100),
    Err(_) => 100,
});

pub struct ITXManager {
    transactions: RwLock<HashMap<TxId, InteractiveTransaction>>,

    /// Cache of closed transactions. We keep the last N closed transactions in memory to
    /// return better error messages if operations are performed on closed transactions.
    closed_txs: RwLock<LruCache<TxId, Option<ClosedTx>>>,
}

impl ITXManager {
    pub fn new() -> Self {
        Self {
            transactions: Default::default(),
            closed_txs: RwLock::new(LruCache::new(*CLOSED_TX_CACHE_SIZE)),
        }
    }

    pub(crate) async fn create_tx(
        &self,
        query_schema: QuerySchemaRef,
        tx_id: TxId,
        conn: Box<dyn Connection + Send + Sync>,
        isolation_level: Option<String>,
        timeout: Duration,
        engine_protocol: EngineProtocol,
    ) -> crate::Result<()> {
        // TODO laplab: maintain dispatcher and span.
        // TODO laplab: monitor timeout.
        // TODO laplab: use `engine_protocol`.

        let transaction =
            InteractiveTransaction::new(tx_id.clone(), conn, timeout, query_schema, isolation_level).await?;

        self.transactions.write().await.insert(tx_id.clone(), transaction);
        Ok(())
    }

    async fn transaction_absent(&self, tx_id: &TxId, from_operation: &str) -> CoreError {
        if let Some(closed_tx) = self.closed_txs.read().await.peek(tx_id) {
            TransactionError::Closed {
                reason: match closed_tx {
                    Some(ClosedTx::Committed) => {
                        format!("A {from_operation} cannot be executed on a committed transaction")
                    }
                    Some(ClosedTx::RolledBack) => {
                        format!("A {from_operation} cannot be executed on a transaction that was rolled back")
                    }
                    Some(ClosedTx::Expired { start_time, timeout }) => {
                        format!(
                            "A {from_operation} cannot be executed on an expired transaction. \
                             The timeout for this transaction was {} ms, however {} ms passed since the start \
                             of the transaction. Consider increasing the interactive transaction timeout \
                             or doing less work in the transaction",
                            timeout.as_millis(),
                            start_time.elapsed_time().as_millis(),
                        )
                    }
                    None => {
                        error!("[{tx_id}] no details about closed transaction");
                        format!("A {from_operation} cannot be executed on a closed transaction")
                    }
                },
            }
            .into()
        } else {
            TransactionError::NotFound.into()
        }
    }

    pub async fn execute(
        &self,
        tx_id: &TxId,
        operation: Operation,
        traceparent: Option<String>,
    ) -> crate::Result<ResponseData> {
        if let Some(transaction) = self.transactions.write().await.get_mut(tx_id) {
            transaction.execute_single(&operation, traceparent).await
        } else {
            Err(self.transaction_absent(tx_id, "query").await)
        }
    }

    pub async fn batch_execute(
        &self,
        tx_id: &TxId,
        operations: Vec<Operation>,
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        if let Some(transaction) = self.transactions.write().await.get_mut(tx_id) {
            transaction.execute_batch(&operations, traceparent).await
        } else {
            Err(self.transaction_absent(tx_id, "batch query").await)
        }
    }

    pub async fn commit_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        if let Some(transaction) = self.transactions.write().await.get_mut(tx_id) {
            transaction.commit().await
        } else {
            Err(self.transaction_absent(tx_id, "commit").await)
        }
    }

    pub async fn rollback_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        if let Some(transaction) = self.transactions.write().await.get_mut(tx_id) {
            transaction.rollback(false).await
        } else {
            Err(self.transaction_absent(tx_id, "rollback").await)
        }
    }
}
