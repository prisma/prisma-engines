use crate::{Operation, ResponseData};
use lru::LruCache;
use once_cell::sync::Lazy;
use schema::QuerySchemaRef;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{
        mpsc::{channel, Sender},
        RwLock,
    },
    task::JoinHandle,
    time::Duration,
};

use super::{spawn_client_list_clear_actor, spawn_itx_actor, ITXClient, OpenTx, TransactionError, TxId};

pub static CLOSED_TX_CACHE_SIZE: Lazy<usize> = Lazy::new(|| match std::env::var("CLOSED_TX_CACHE_SIZE") {
    Ok(size) => size.parse().unwrap_or(100),
    Err(_) => 100,
});

static CHANNEL_SIZE: usize = 100;

pub struct TransactionActorManager {
    /// Map of active ITx clients
    pub clients: Arc<RwLock<HashMap<TxId, ITXClient>>>,
    /// Cache of closed transactions. We keep the last N closed transactions in memory to
    /// return better error messages if operations are performed on closed transactions.
    pub closed_txs: Arc<RwLock<LruCache<TxId, ()>>>,
    /// Channel used to signal an ITx is closed and can be moved to the list of closed transactions.
    send_done: Sender<TxId>,
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
        let clients: Arc<RwLock<HashMap<TxId, ITXClient>>> = Arc::new(RwLock::new(HashMap::new()));
        let closed_txs: Arc<RwLock<LruCache<TxId, ()>>> = Arc::new(RwLock::new(LruCache::new(*CLOSED_TX_CACHE_SIZE)));

        let (send_done, rx) = channel::<TxId>(CHANNEL_SIZE);
        let handle = spawn_client_list_clear_actor(clients.clone(), closed_txs.clone(), rx);

        Self {
            clients,
            closed_txs,
            send_done,
            bg_reader_clear: handle,
        }
    }

    pub async fn create_tx(&self, query_schema: QuerySchemaRef, tx_id: TxId, value: OpenTx, timeout: Duration) {
        let client = spawn_itx_actor(
            query_schema.clone(),
            tx_id.clone(),
            value,
            timeout,
            CHANNEL_SIZE,
            self.send_done.clone(),
        );

        self.clients.write().await.insert(tx_id, client);
    }

    async fn get_client(&self, tx_id: &TxId, from_operation: &str) -> crate::Result<ITXClient> {
        if let Some(client) = self.clients.read().await.get(tx_id) {
            Ok(client.clone())
        } else if self.closed_txs.read().await.contains(tx_id) {
            Err(TransactionError::Closed {
                reason: format!("A {from_operation} cannot be executed on a closed transaction."),
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
        trace_id: Option<String>,
    ) -> crate::Result<ResponseData> {
        let client = self.get_client(tx_id, "query").await?;

        client.execute(operation, trace_id).await
    }

    pub async fn batch_execute(
        &self,
        tx_id: &TxId,
        operations: Vec<Operation>,
        trace_id: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        let client = self.get_client(tx_id, "batch query").await?;

        client.batch_execute(operations, trace_id).await
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
