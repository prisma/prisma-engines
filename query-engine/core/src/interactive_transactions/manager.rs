use crate::{ClosedTx, InteractiveTransaction, Operation, ResponseData};
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
    /// Stores all current transactions (some of them might be already committed/expired/rolled back).
    ///
    /// There are two tiers of locks here:
    ///  1. Lock on the entire hashmap. This *must* be taken only for short periods of time - for
    ///     example to insert/delete transaction or to clone transaction inside.
    ///  2. Lock on the individual transactins. This one can be taken for prolonged periods of time - for
    ///     example to perform an I/O operation.
    ///
    /// The rationale behind this design is to make shared path (lock on the entire hashmap) as free
    /// from contention as possible. Individual transactions are not capable of concurrency, so
    /// taking a lock on them to serialise operations is acceptable.
    ///
    /// Note that since we clone transaction from the shared hashmap to perform operations on it, it
    /// is possible to end up in a situation where we cloned the transaction, but it was then
    /// immediately removed by the background task from the common hashmap. In this case, either
    /// our operation will be first or the background cleanup task will be first. Both cases are
    /// an acceptable outcome.
    transactions: Arc<RwLock<HashMap<TxId, Arc<RwLock<InteractiveTransaction>>>>>,

    /// Cache of closed transactions. We keep the last N closed transactions in memory to
    /// return better error messages if operations are performed on closed transactions.
    closed_txs: Arc<RwLock<LruCache<TxId, ClosedTx>>>,

    timeout_sender: UnboundedSender<TxId>,
}

impl ITXManager {
    pub fn new() -> Self {
        let transactions: Arc<RwLock<HashMap<TxId, Arc<RwLock<InteractiveTransaction>>>>> =
            Arc::new(RwLock::new(HashMap::default()));
        let closed_txs = Arc::new(RwLock::new(LruCache::new(*CLOSED_TX_CACHE_SIZE)));
        let (timeout_sender, mut timeout_receiver) = unbounded_channel();

        crosstarget_utils::task::spawn({
            let transactions = transactions.clone();
            let closed_txs = closed_txs.clone();
            async move {
                while let Some(tx_id) = timeout_receiver.recv().await {
                    let closed_tx = {
                        let transaction_entry = transactions.read().await.get(&tx_id).expect("invalid tx_id").clone();
                        let mut transaction = transaction_entry.write().await;

                        // If transaction was already committed, rollback will error.
                        let _ = transaction.rollback(true).await;

                        transaction
                            .to_closed()
                            .expect("transaction must be closed after rollback")
                    };

                    transactions.write().await.remove(&tx_id);
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

        self.transactions
            .write()
            .await
            .insert(tx_id, Arc::new(RwLock::new(transaction)));
        Ok(())
    }

    async fn get_transaction(
        &self,
        tx_id: &TxId,
        from_operation: &str,
    ) -> crate::Result<Arc<RwLock<InteractiveTransaction>>> {
        if let Some(transaction) = self.transactions.read().await.get(tx_id) {
            Ok(transaction.clone())
        } else {
            Err(if let Some(closed_tx) = self.closed_txs.read().await.peek(tx_id) {
                TransactionError::Closed {
                    reason: closed_tx.error_message_for(from_operation),
                }
                .into()
            } else {
                TransactionError::NotFound.into()
            })
        }
    }

    pub async fn execute(
        &self,
        tx_id: &TxId,
        operation: Operation,
        traceparent: Option<String>,
    ) -> crate::Result<ResponseData> {
        self.get_transaction(tx_id, "query")
            .await?
            .write()
            .await
            .execute_single(&operation, traceparent)
            .await
    }

    pub async fn batch_execute(
        &self,
        tx_id: &TxId,
        operations: Vec<Operation>,
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        self.get_transaction(tx_id, "batch query")
            .await?
            .write()
            .await
            .execute_batch(&operations, traceparent)
            .await
    }

    pub async fn commit_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        self.get_transaction(tx_id, "commit")
            .await?
            .write()
            .await
            .commit()
            .await
    }

    pub async fn rollback_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        self.get_transaction(tx_id, "rollback")
            .await?
            .write()
            .await
            .rollback(false)
            .await
    }
}
