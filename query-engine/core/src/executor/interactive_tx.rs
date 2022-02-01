use super::{
    execute_operation::{execute_many_operations, execute_single_operation},
    pipeline::QueryPipeline,
};
use crate::{CoreError, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, ResponseData};
use connector::{Connection, ConnectionLike, Transaction};
use once_cell::sync::Lazy;
use std::{collections::HashMap, fmt::Display, sync::Arc};
use thiserror::Error;
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot, RwLock,
    },
    task::JoinHandle,
    time::{self, Duration},
};

use crate::Operation;

/*
How Interactive Transactions work
The Interactive Transactions (iTx) follow an actor model design. Where each iTx is created in its own process.
When a prisma client requests to start a new transaction, the Transaction Manager spawns a new ITXServer. The ITXServer runs in its own
process and waits for messages to arrive via its receive channel to process.
The iTx Manager will also create an ITXClient and add it to hashmap managed by an RwLock. The iTx client is the only way to communicate
with the ITXServer.

Once the prisma client receives the iTx Id it can perform database operations using that iTx id. When an operation request is received by the
TransactionManager, it looks for the client in the hashmap and passes the operation to the client. The iTx client sends a message to the
iTx server and waits for a response. The ITXServer will then perform the operation and return the result. The ITXServer will perform one
operation at a time. All other operations will sit in the message queue waiting to be processed.
The ITXServer will handle all messages until it transitions state, e.g "rollback" or "commit".

After that the ITXServer will move into the cache eviction state. In this state, the connection is closed, and any messages it receives, the will
will only reply with its last state. i.e committed, rollbacked or timeout. The eviction state is there so that if a prisma wants to
perform an action on a iTx that has completed it will get a better message rather than the error message that this transaction doesn't exist

Once the eviction timeout is exceeded, the ITXServer will send a message to the Background Client list process to say that it is completed,
and the ITXServer will end. The Background Client list process removes the client from the list of clients that are active.

During the time the ITXServer is active there is a timer running and if that timeout is exceeded, the
transaction is rolledback and the connection to the database is closed. The ITXServer will then move into the eviction state.
*/

pub static CACHE_EVICTION_SECS: Lazy<u64> = Lazy::new(|| match std::env::var("CLOSED_TX_CLEANUP") {
    Ok(size) => size.parse().unwrap_or(300),
    Err(_) => 300,
});

static CHANNEL_SIZE: usize = 100;

#[derive(Debug, Error, PartialEq)]
pub enum TransactionError {
    #[error("Unable to start a transaction in the given time.")]
    AcquisitionTimeout,

    #[error("Attempted to start a transaction inside of a transaction.")]
    AlreadyStarted,

    #[error("Transaction not found.")]
    NotFound,

    #[error("Transaction already closed: {reason}.")]
    Closed { reason: String },

    #[error("Unexpected response: {reason}.")]
    Unknown { reason: String },
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TxId(String);

impl Default for TxId {
    fn default() -> Self {
        Self(cuid::cuid().unwrap())
    }
}

impl<T> From<T> for TxId
where
    T: Into<String>,
{
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

impl Display for TxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum TxOpRequestMsg {
    Commit,
    Rollback,
    Single(Operation, Option<String>),
    Batch(Vec<Operation>, Option<String>),
}

pub struct TxOpRequest {
    msg: TxOpRequestMsg,
    respond_to: oneshot::Sender<TxOpResponse>,
}

impl Display for TxOpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.msg {
            TxOpRequestMsg::Commit => write!(f, "Commit"),
            TxOpRequestMsg::Rollback => write!(f, "Rollback"),
            TxOpRequestMsg::Single(..) => write!(f, "Single"),
            TxOpRequestMsg::Batch(..) => write!(f, "Batch"),
        }
    }
}

#[derive(Debug)]
pub enum TxOpResponse {
    Committed(crate::Result<()>),
    RolledBack(crate::Result<()>),
    Expired,
    Single(crate::Result<ResponseData>),
    Batch(crate::Result<Vec<crate::Result<ResponseData>>>),
}

impl Display for TxOpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Committed(..) => write!(f, "Committed"),
            Self::RolledBack(..) => write!(f, "RolledBack"),
            Self::Expired => write!(f, "Expired"),
            Self::Single(..) => write!(f, "Single"),
            Self::Batch(..) => write!(f, "Single"),
        }
    }
}

pub struct ITXServer {
    id: TxId,
    pub cached_tx: CachedTx,
    pub timeout: Duration,
    receive: Receiver<TxOpRequest>,
    query_schema: QuerySchemaRef,
}

impl ITXServer {
    pub fn new(
        id: TxId,
        tx: CachedTx,
        timeout: Duration,
        receive: Receiver<TxOpRequest>,
        query_schema: QuerySchemaRef,
    ) -> Self {
        Self {
            id,
            cached_tx: tx,
            timeout,
            receive,
            query_schema,
        }
    }

    // The bool returned notifies the actor loop if the process should continue receiving msg's
    // or if it should finish. `true` means the process is finished.
    pub async fn process_msg(&mut self, op: TxOpRequest) -> bool {
        let is_finished = true;
        let should_continue = false;
        match op.msg {
            TxOpRequestMsg::Single(ref operation, trace_id) => {
                let result = self.execute_single(&operation, trace_id).await;
                let _ = op.respond_to.send(TxOpResponse::Single(result));
                should_continue
            }
            TxOpRequestMsg::Batch(ref operations, trace_id) => {
                let result = self.execute_batch(&operations, trace_id).await;
                let _ = op.respond_to.send(TxOpResponse::Batch(result));
                should_continue
            }
            TxOpRequestMsg::Commit => {
                let resp = self.commit().await;
                let _ = op.respond_to.send(TxOpResponse::Committed(resp));
                is_finished
            }
            TxOpRequestMsg::Rollback => {
                let resp = self.rollback(false).await;
                let _ = op.respond_to.send(TxOpResponse::RolledBack(resp));
                is_finished
            }
        }
    }

    #[tracing::instrument(skip(self, operation))]
    async fn execute_single(&mut self, operation: &Operation, trace_id: Option<String>) -> crate::Result<ResponseData> {
        let conn = self.cached_tx.as_open()?;
        execute_single_operation(
            self.query_schema.clone(),
            conn.as_connection_like(),
            operation,
            trace_id,
        )
        .await
    }

    #[tracing::instrument(skip(self, operations))]
    async fn execute_batch(
        &mut self,
        operations: &[Operation],
        trace_id: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        let conn = self.cached_tx.as_open()?;
        execute_many_operations(
            self.query_schema.clone(),
            conn.as_connection_like(),
            operations,
            trace_id,
        )
        .await
    }

    pub async fn commit(&mut self) -> crate::Result<()> {
        if let CachedTx::Open(_) = self.cached_tx {
            let open_tx = self.cached_tx.as_open()?;
            debug!("[{}] committing.", self.id.to_string());
            open_tx.tx.commit().await?;
            self.cached_tx = CachedTx::Committed;
        }

        Ok(())
    }

    pub async fn rollback(&mut self, was_timeout: bool) -> crate::Result<()> {
        debug!("[{}] {was_timeout} rolling back", self.name());
        if let CachedTx::Open(_) = self.cached_tx {
            let open_tx = self.cached_tx.as_open()?;
            open_tx.tx.rollback().await?;
            if was_timeout {
                debug!("[{}] Expired Rolling back", self.id.to_string());
                self.cached_tx = CachedTx::Expired;
            } else {
                self.cached_tx = CachedTx::RolledBack;
                debug!("[{}] Rolling back", self.id.to_string());
            }
        }

        Ok(())
    }

    pub fn name(&self) -> String {
        format!("itx-{:?}", self.id.to_string())
    }
}

pub struct ITXClient {
    send: Sender<TxOpRequest>,
    tx_id: TxId,
}

impl ITXClient {
    async fn commit(&self) -> crate::Result<()> {
        let msg = self.send_and_receive(TxOpRequestMsg::Commit).await?;

        if let TxOpResponse::Committed(resp) = msg {
            debug!("[{}] COMMITTED {:?}", self.tx_id, resp);
            resp
        } else {
            Err(self.handle_error(msg).into())
        }
    }

    async fn rollback(&self) -> crate::Result<()> {
        let msg = self.send_and_receive(TxOpRequestMsg::Rollback).await?;

        if let TxOpResponse::RolledBack(resp) = msg {
            resp
        } else {
            Err(self.handle_error(msg).into())
        }
    }

    async fn execute(&self, operation: Operation, trace_id: Option<String>) -> crate::Result<ResponseData> {
        let msg_req = TxOpRequestMsg::Single(operation, trace_id);
        let msg = self.send_and_receive(msg_req).await?;

        if let TxOpResponse::Single(resp) = msg {
            resp
        } else {
            Err(self.handle_error(msg).into())
        }
    }

    async fn batch_execute(
        &self,
        operations: Vec<Operation>,
        trace_id: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        let msg_req = TxOpRequestMsg::Batch(operations, trace_id);

        let msg = self.send_and_receive(msg_req).await?;

        if let TxOpResponse::Batch(resp) = msg {
            resp
        } else {
            Err(self.handle_error(msg).into())
        }
    }

    async fn send_and_receive(&self, msg: TxOpRequestMsg) -> Result<TxOpResponse, crate::CoreError> {
        let (receiver, req) = self.create_receive_and_req(msg);
        if let Err(err) = self.send.send(req).await {
            debug!("channel send error {err}");
            return Err(TransactionError::Closed {
                reason: "Cound not perform operation".to_string(),
            }
            .into());
        }

        match receiver.await {
            Ok(resp) => Ok(resp),
            Err(_err) => Err(TransactionError::Closed {
                reason: "Cound not perform operation".to_string(),
            }
            .into()),
        }
    }

    fn create_receive_and_req(&self, msg: TxOpRequestMsg) -> (oneshot::Receiver<TxOpResponse>, TxOpRequest) {
        let (send, rx) = oneshot::channel::<TxOpResponse>();
        let request = TxOpRequest { msg, respond_to: send };
        (rx, request)
    }

    fn handle_error(&self, msg: TxOpResponse) -> TransactionError {
        match msg {
            TxOpResponse::Expired => {
                let reason = "Transaction is no longer valid. Last state: 'Expired'".to_string();
                TransactionError::Closed { reason }
            }
            TxOpResponse::Committed(..) => {
                let reason = "Transaction is no longer valid. Last state: 'Committed'".to_string();
                TransactionError::Closed { reason }
            }
            TxOpResponse::RolledBack(..) => {
                let reason = "Transaction is no longer valid. Last state: 'RolledBack'".to_string();
                TransactionError::Closed { reason }
            }
            other => {
                error!("Unexpected iTx response, {}", other);
                let reason = format!("response '{}'", other);
                TransactionError::Closed { reason }
            }
        }
    }
}

pub(crate) struct TransactionProcessManager {
    pub clients: Arc<RwLock<HashMap<TxId, ITXClient>>>,
    send_done: Sender<TxId>,
    bg_reader_clear: JoinHandle<()>,
}

impl Drop for TransactionProcessManager {
    fn drop(&mut self) {
        debug!("DROPPING TPM");
        self.bg_reader_clear.abort();
    }
}

impl TransactionProcessManager {
    pub fn new() -> Self {
        let clients: Arc<RwLock<HashMap<TxId, ITXClient>>> = Arc::new(RwLock::new(HashMap::new()));

        let (send_done, mut rx) = channel::<TxId>(CHANNEL_SIZE);
        let c = clients.clone();
        let handle = tokio::task::spawn(async move {
            loop {
                if let Some(id) = rx.recv().await {
                    debug!("removing {} from client list", id);
                    c.write().await.remove(&id);
                }
            }
        });

        Self {
            clients,
            send_done,
            bg_reader_clear: handle,
        }
    }

    pub async fn create_tx(&self, query_schema: QuerySchemaRef, tx_id: TxId, value: OpenTx, timeout: Duration) {
        let (tx_to_server, rx_from_client) = channel::<TxOpRequest>(CHANNEL_SIZE);

        let client = ITXClient {
            send: tx_to_server,
            tx_id: tx_id.clone(),
        };

        self.clients.write().await.insert(tx_id.clone(), client);

        let mut server = ITXServer::new(tx_id, CachedTx::Open(value), timeout, rx_from_client, query_schema);
        let send_done = self.send_done.clone();

        tokio::task::spawn(async move {
            let sleep = time::sleep(timeout);
            tokio::pin!(sleep);

            loop {
                tokio::select! {
                    _ = &mut sleep => {
                        debug!("[{}] interactive transaction timed out", server.id.to_string());
                        let _ = server.rollback(true).await;
                        break;
                    }
                    msg = server.receive.recv() => {
                        if let Some(op) = msg {
                            let finished = server.process_msg(op).await;

                            if finished {
                                break
                            }
                        }
                    }
                }
            }

            debug!("[{}] completed with {}", server.id.to_string(), server.cached_tx);
            let eviction_sleep = time::sleep(Duration::from_secs(*CACHE_EVICTION_SECS));
            tokio::pin!(eviction_sleep);

            loop {
                tokio::select! {
                    _ = &mut eviction_sleep => {
                        break;
                    }
                    msg = server.receive.recv() => {
                        if let Some(op) = msg {
                            let msg = match server.cached_tx {
                                CachedTx::Committed => TxOpResponse::Committed(Ok(())),
                                CachedTx::RolledBack => TxOpResponse::RolledBack(Ok(())),
                                CachedTx::Expired => TxOpResponse::Expired,
                                 _ => {
                                     error!("[{}] unexpected state {}", server.id.to_string(), server.cached_tx);
                                     let _ = server.rollback(true).await;
                                     let msg = "The transaction was in an unexpected state and rolledback".to_string();
                                     let err = Err(TransactionError::Unknown{ reason: msg }.into());
                                     TxOpResponse::RolledBack(err)
                                 }
                                };

                                // we ignore any errors when sending
                                let _ = op.respond_to.send(msg);
                            }
                        }
                }
            }

            let _ = send_done.send(server.id.clone()).await;
            debug!("[{}] has stopped with {}", server.id.to_string(), server.cached_tx);
        });
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

pub enum CachedTx {
    Open(OpenTx),
    Aborted,
    Committed,
    RolledBack,
    Expired,
}

impl Display for CachedTx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CachedTx::Open(_) => write!(f, "Open"),
            CachedTx::Aborted => write!(f, "Aborted"),
            CachedTx::Committed => write!(f, "Committed"),
            CachedTx::RolledBack => write!(f, "Rolled back"),
            CachedTx::Expired => write!(f, "Expired"),
        }
    }
}

impl CachedTx {
    /// Requires this cached TX to be `Open`, else an error will be raised that it is no longer valid.
    /// Consumes self to remove the `CachedTx` indirection to get to the underlying `OpenTx`.
    pub fn into_open(self) -> crate::Result<OpenTx> {
        if let Self::Open(otx) = self {
            Ok(otx)
        } else {
            let reason = format!("Transaction is no longer valid. Last state: '{}'", self);
            Err(CoreError::from(TransactionError::Closed { reason }))
        }
    }

    /// Requires this cached TX to be `Open`, else an error will be raised that it is no longer valid.
    pub fn as_open(&mut self) -> crate::Result<&mut OpenTx> {
        if let Self::Open(ref mut otx) = self {
            Ok(otx)
        } else {
            let reason = format!("Transaction is no longer valid. Last state: '{}'", self);
            Err(CoreError::from(TransactionError::Closed { reason }))
        }
    }
}

pub struct OpenTx {
    pub conn: Box<dyn Connection>,
    pub tx: Box<dyn Transaction + 'static>,
    pub expiration_timer: Option<JoinHandle<()>>,
}

impl OpenTx {
    pub async fn start(mut conn: Box<dyn Connection>) -> crate::Result<Self> {
        // Forces static lifetime for the transaction, disabling the lifetime checks for `tx`.
        // Why is this okay? We store the connection the tx depends on with its lifetime next to
        // the tx in the struct. Neither the connection nor the tx are moved out of this struct.
        // The `OpenTx` struct is dropped as a unit.
        let transaction: Box<dyn Transaction + '_> = conn.start_transaction().await?;
        let tx = unsafe {
            let tx: Box<dyn Transaction + 'static> = std::mem::transmute(transaction);
            tx
        };

        let c_tx = OpenTx {
            conn,
            tx,
            expiration_timer: None,
        };

        Ok(c_tx)
    }

    pub fn as_connection_like(&mut self) -> &mut dyn ConnectionLike {
        self.tx.as_mut().as_connection_like()
    }
}

impl Into<CachedTx> for OpenTx {
    fn into(self) -> CachedTx {
        CachedTx::Open(self)
    }
}
