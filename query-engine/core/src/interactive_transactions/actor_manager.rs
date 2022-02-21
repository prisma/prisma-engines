use crate::{Operation, QuerySchemaRef, ResponseData};
use once_cell::sync::Lazy;
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

pub static CACHE_EVICTION_SECS: Lazy<u64> = Lazy::new(|| match std::env::var("CLOSED_TX_CLEANUP") {
    Ok(size) => size.parse().unwrap_or(300),
    Err(_) => 300,
});

static CHANNEL_SIZE: usize = 100;

pub struct TransactionActorManager {
    pub clients: Arc<RwLock<HashMap<TxId, ITXClient>>>,
    send_done: Sender<TxId>,
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

        let (send_done, rx) = channel::<TxId>(CHANNEL_SIZE);
        let handle = spawn_client_list_clear_actor(clients.clone(), rx);

        Self {
            clients,
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
            *CACHE_EVICTION_SECS,
            self.send_done.clone(),
        );

        self.clients.write().await.insert(tx_id, client);
    }

    pub async fn execute(
        &self,
        tx_id: &TxId,
        operation: Operation,
        trace_id: Option<String>,
    ) -> crate::Result<ResponseData> {
        if let Some(client) = self.clients.read().await.get(tx_id) {
            let resp = client.execute(operation, trace_id).await;
            resp
        } else {
            Err(TransactionError::NotFound.into())
        }
    }

    pub async fn batch_execute(
        &self,
        tx_id: &TxId,
        operations: Vec<Operation>,
        trace_id: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        if let Some(client) = self.clients.read().await.get(tx_id) {
            client.batch_execute(operations, trace_id).await
        } else {
            Err(TransactionError::NotFound.into())
        }
    }

    pub async fn commit_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        if let Some(client) = self.clients.read().await.get(tx_id) {
            client.commit().await?;
            Ok(())
        } else {
            Err(TransactionError::NotFound.into())
        }
    }

    pub async fn rollback_tx(&self, tx_id: &TxId) -> crate::Result<()> {
        if let Some(client) = self.clients.read().await.get(tx_id) {
            client.rollback().await?;
            Ok(())
        } else {
            Err(TransactionError::NotFound.into())
        }
    }
}
