use crate::{ClosedTx, CoreError, InteractiveTransaction, Operation, ResponseData};
use connector::Connection;
use lru::LruCache;
use once_cell::sync::Lazy;
use schema::QuerySchemaRef;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        RwLock,
    },
    time::Duration,
};

use super::{TransactionError, TxId};

pub static CLOSED_TX_CACHE_SIZE: Lazy<usize> = Lazy::new(|| match std::env::var("CLOSED_TX_CACHE_SIZE") {
    Ok(size) => size.parse().unwrap_or(100),
    Err(_) => 100,
});

pub struct ITXManager {
    transactions: Arc<RwLock<HashMap<TxId, InteractiveTransaction>>>,

    /// Cache of closed transactions. We keep the last N closed transactions in memory to
    /// return better error messages if operations are performed on closed transactions.
    closed_txs: Arc<RwLock<LruCache<TxId, ClosedTx>>>,

    timeout_sender: UnboundedSender<TxId>,
}

impl ITXManager {
    pub fn new() -> Self {
        let transactions = Arc::new(RwLock::new(HashMap::default()));
        let closed_txs = Arc::new(RwLock::new(LruCache::new(*CLOSED_TX_CACHE_SIZE)));
        let (timeout_sender, mut timeout_receiver) = unbounded_channel();

        crosstarget_utils::task::spawn({
            let transactions = transactions.clone();
            let closed_txs = closed_txs.clone();
            async move {
                while let Some(tx_id) = timeout_receiver.recv().await {
                    let mut transactions = transactions.write().await;
                    let closed_tx = {
                        let transaction: &mut InteractiveTransaction =
                            transactions.get_mut(&tx_id).expect("invalid tx_id");

                        // If transaction was already committed, rollback will be ignored.
                        let _ = transaction.rollback(true).await;

                        transaction
                            .to_closed()
                            .expect("transaction must be closed after rollback")
                    };

                    transactions.remove(&tx_id);
                    closed_txs.write().await.put(tx_id, closed_tx);
                }
            }
        });

        Self {
            transactions,
            closed_txs,
            timeout_sender,
        }
    }

    pub(crate) async fn create_tx(
        &self,
        query_schema: QuerySchemaRef,
        tx_id: TxId,
        conn: Box<dyn Connection + Send + Sync>,
        isolation_level: Option<String>,
        timeout: Duration,
    ) -> crate::Result<()> {
        // TODO laplab: start a background task to clear stale transactions.

        crosstarget_utils::task::spawn({
            let timeout_sender = self.timeout_sender.clone();
            let tx_id = tx_id.clone();
            async move {
                crosstarget_utils::time::sleep(timeout).await;
                timeout_sender.send(tx_id).expect("receiver must exist");
            }
        });

        let transaction =
            InteractiveTransaction::new(tx_id.clone(), conn, timeout, query_schema, isolation_level).await?;

        self.transactions.write().await.insert(tx_id, transaction);
        Ok(())
    }

    async fn transaction_absent(&self, tx_id: &TxId, from_operation: &str) -> CoreError {
        if let Some(closed_tx) = self.closed_txs.read().await.peek(tx_id) {
            TransactionError::Closed {
                reason: match closed_tx {
                    ClosedTx::Committed => {
                        format!("A {from_operation} cannot be executed on a committed transaction")
                    }
                    ClosedTx::RolledBack => {
                        format!("A {from_operation} cannot be executed on a transaction that was rolled back")
                    }
                    ClosedTx::Expired { start_time, timeout } => {
                        format!(
                            "A {from_operation} cannot be executed on an expired transaction. \
                             The timeout for this transaction was {} ms, however {} ms passed since the start \
                             of the transaction. Consider increasing the interactive transaction timeout \
                             or doing less work in the transaction",
                            timeout.as_millis(),
                            start_time.elapsed_time().as_millis(),
                        )
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
