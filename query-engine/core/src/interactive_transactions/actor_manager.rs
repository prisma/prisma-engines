use crate::executor::task::JoinHandle;
use crate::{protocol::EngineProtocol, ClosedTx, Operation, ResponseData};
use connector::Connection;
use lru::LruCache;
use once_cell::sync::Lazy;
use schema::QuerySchemaRef;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{
        mpsc::{channel, Sender},
        RwLock,
    },
    time::Duration,
};

use super::{spawn_client_list_clear_actor, spawn_itx_actor, ITXClient, TransactionError, TxId};

pub static CLOSED_TX_CACHE_SIZE: Lazy<usize> = Lazy::new(|| match std::env::var("CLOSED_TX_CACHE_SIZE") {
    Ok(size) => size.parse().unwrap_or(100),
    Err(_) => 100,
});

static CHANNEL_SIZE: usize = 100;

pub struct TransactionActorManager {
    /// Map of active ITx clients
    pub(crate) clients: Arc<RwLock<HashMap<TxId, ITXClient>>>,
    /// Cache of closed transactions. We keep the last N closed transactions in memory to
    /// return better error messages if operations are performed on closed transactions.
    pub(crate) closed_txs: Arc<RwLock<LruCache<TxId, Option<ClosedTx>>>>,
    /// Channel used to signal an ITx is closed and can be moved to the list of closed transactions.
    send_done: Sender<(TxId, Option<ClosedTx>)>,
    /// Handle to the task in charge of clearing actors.
    /// Used to abort the task when the TransactionActorManager is dropped.
    bg_reader_clear: JoinHandle<()>,
}

impl Drop for TransactionActorManager {
    fn drop(&mut self) {
        debug!("DROPPING TPM");
        self.bg_reader_clear.abort();
    }
}

impl Default for TransactionActorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionActorManager {
    pub fn new() -> Self {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let closed_txs = Arc::new(RwLock::new(LruCache::new(*CLOSED_TX_CACHE_SIZE)));

        let (send_done, rx) = channel(CHANNEL_SIZE);
        let handle = spawn_client_list_clear_actor(clients.clone(), closed_txs.clone(), rx);

        Self {
            clients,
            closed_txs,
            send_done,
            bg_reader_clear: handle,
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
        // Only create a client if there is no client for this transaction yet.
        // otherwise, begin a new transaction/savepoint for the existing client.
        if !self.clients.read().await.contains_key(&tx_id) {
            let client = spawn_itx_actor(
                query_schema.clone(),
                tx_id.clone(),
                conn,
                isolation_level,
                timeout,
                CHANNEL_SIZE,
                self.send_done.clone(),
                engine_protocol,
            )
            .await?;

            self.clients.write().await.insert(tx_id, client);
        } else {
            let client = self.get_client(&tx_id, "begin").await?;
            client.begin().await?;
        }

        Ok(())
    }

    async fn get_client(&self, tx_id: &TxId, from_operation: &str) -> crate::Result<ITXClient> {
        if let Some(client) = self.clients.read().await.get(tx_id) {
            Ok(client.clone())
        } else if let Some(closed_tx) = self.closed_txs.read().await.peek(tx_id) {
            Err(TransactionError::Closed {
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
                            start_time.elapsed().as_millis(),
                        )
                    }
                    None => {
                        error!("[{tx_id}] no details about closed transaction");
                        format!("A {from_operation} cannot be executed on a closed transaction")
                    }
                },
            }
            .into())
        } else {
            Err(TransactionError::NotFound.into())
        }
    }

    pub async fn execute(
        &self,
        tx_id: &TxId,
        operation: Operation,
        traceparent: Option<String>,
    ) -> crate::Result<ResponseData> {
        let client = self.get_client(tx_id, "query").await?;

        client.execute(operation, traceparent).await
    }

    pub async fn batch_execute(
        &self,
        tx_id: &TxId,
        operations: Vec<Operation>,
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        let client = self.get_client(tx_id, "batch query").await?;

        client.batch_execute(operations, traceparent).await
    }

    pub async fn commit_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        let client = self.get_client(tx_id, "commit").await?;
        client.commit().await?;

        Ok(())
    }

    pub async fn rollback_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        let client = self.get_client(tx_id, "rollback").await?;
        client.rollback().await?;

        Ok(())
    }
}
