use super::{CachedTx, TransactionError, TxOpRequest, TxOpRequestMsg, TxOpResponse};
use crate::{
    execute_many_operations, execute_single_operation, protocol::EngineProtocol,
    telemetry::helpers::set_span_link_from_traceparent, ClosedTx, OpenTx, Operation, ResponseData, TxId,
};
use schema::QuerySchemaRef;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot, RwLock,
    },
    task::JoinHandle,
    time::{self, Duration, Instant},
};
use tracing::Span;
use tracing_futures::Instrument;
use tracing_futures::WithSubscriber;

#[derive(PartialEq)]
enum RunState {
    Continue,
    Finished,
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

    // RunState is used to tell if the run loop should continue
    async fn process_msg(&mut self, op: TxOpRequest) -> RunState {
        match op.msg {
            TxOpRequestMsg::Single(ref operation, traceparent) => {
                let result = self.execute_single(&operation, traceparent).await;
                let _ = op.respond_to.send(TxOpResponse::Single(result));
                RunState::Continue
            }
            TxOpRequestMsg::Batch(ref operations, traceparent) => {
                let result = self.execute_batch(&operations, traceparent).await;
                let _ = op.respond_to.send(TxOpResponse::Batch(result));
                RunState::Continue
            }
            TxOpRequestMsg::Commit => {
                let resp = self.commit().await;
                let _ = op.respond_to.send(TxOpResponse::Committed(resp));
                RunState::Finished
            }
            TxOpRequestMsg::Rollback => {
                let resp = self.rollback(false).await;
                let _ = op.respond_to.send(TxOpResponse::RolledBack(resp));
                RunState::Finished
            }
        }
    }

    async fn execute_single(
        &mut self,
        operation: &Operation,
        traceparent: Option<String>,
    ) -> crate::Result<ResponseData> {
        let span = info_span!("prisma:engine:itx_query_builder", user_facing = true);
        set_span_link_from_traceparent(&span, traceparent.clone());

        let conn = self.cached_tx.as_open()?;
        execute_single_operation(
            self.query_schema.clone(),
            conn.as_connection_like(),
            operation,
            traceparent,
        )
        .instrument(span)
        .await
    }

    async fn execute_batch(
        &mut self,
        operations: &[Operation],
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        let span = info_span!("prisma:engine:itx_execute", user_facing = true);

        let conn = self.cached_tx.as_open()?;
        execute_many_operations(
            self.query_schema.clone(),
            conn.as_connection_like(),
            operations,
            traceparent,
        )
        .instrument(span)
        .await
    }

    pub async fn commit(&mut self) -> crate::Result<()> {
        if let CachedTx::Open(_) = self.cached_tx {
            let open_tx = self.cached_tx.as_open()?;
            trace!("[{}] committing.", self.id.to_string());
            open_tx.tx.commit().await?;
            self.cached_tx = CachedTx::Committed;
        }

        Ok(())
    }

    pub async fn rollback(&mut self, was_timeout: bool) -> crate::Result<()> {
        debug!("[{}] rolling back, was timed out = {was_timeout}", self.name());
        if let CachedTx::Open(_) = self.cached_tx {
            let open_tx = self.cached_tx.as_open()?;
            open_tx.tx.rollback().await?;
            if was_timeout {
                trace!("[{}] Expired Rolling back", self.id.to_string());
                self.cached_tx = CachedTx::Expired;
            } else {
                self.cached_tx = CachedTx::RolledBack;
                trace!("[{}] Rolling back", self.id.to_string());
            }
        }

        Ok(())
    }

    pub fn name(&self) -> String {
        format!("itx-{:?}", self.id.to_string())
    }
}

#[derive(Clone)]
pub struct ITXClient {
    send: Sender<TxOpRequest>,
    tx_id: TxId,
}

impl ITXClient {
    pub async fn commit(&self) -> crate::Result<()> {
        let msg = self.send_and_receive(TxOpRequestMsg::Commit).await?;

        if let TxOpResponse::Committed(resp) = msg {
            debug!("[{}] COMMITTED {:?}", self.tx_id, resp);
            resp
        } else {
            Err(self.handle_error(msg).into())
        }
    }

    pub async fn rollback(&self) -> crate::Result<()> {
        let msg = self.send_and_receive(TxOpRequestMsg::Rollback).await?;

        if let TxOpResponse::RolledBack(resp) = msg {
            resp
        } else {
            Err(self.handle_error(msg).into())
        }
    }

    pub async fn execute(&self, operation: Operation, traceparent: Option<String>) -> crate::Result<ResponseData> {
        let msg_req = TxOpRequestMsg::Single(operation, traceparent);
        let msg = self.send_and_receive(msg_req).await?;

        if let TxOpResponse::Single(resp) = msg {
            resp
        } else {
            Err(self.handle_error(msg).into())
        }
    }

    pub async fn batch_execute(
        &self,
        operations: Vec<Operation>,
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        let msg_req = TxOpRequestMsg::Batch(operations, traceparent);

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
                reason: "Could not perform operation".to_string(),
            }
            .into());
        }

        match receiver.await {
            Ok(resp) => Ok(resp),
            Err(_err) => Err(TransactionError::Closed {
                reason: "Could not perform operation".to_string(),
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
                let reason = format!("response '{other}'");
                TransactionError::Closed { reason }
            }
        }
    }
}

pub fn spawn_itx_actor(
    query_schema: QuerySchemaRef,
    tx_id: TxId,
    value: OpenTx,
    timeout: Duration,
    channel_size: usize,
    send_done: Sender<(TxId, Option<ClosedTx>)>,
    engine_protocol: EngineProtocol,
) -> ITXClient {
    let (tx_to_server, rx_from_client) = channel::<TxOpRequest>(channel_size);

    let client = ITXClient {
        send: tx_to_server,
        tx_id: tx_id.clone(),
    };

    let mut server = ITXServer::new(
        tx_id.clone(),
        CachedTx::Open(value),
        timeout,
        rx_from_client,
        query_schema,
    );
    let dispatcher = crate::get_current_dispatcher();
    let span = Span::current();

    let tx_id_str = tx_id.to_string();
    span.record("itx_id", &tx_id_str.as_str());

    tokio::task::spawn(
        crate::executor::with_request_context(engine_protocol, async move {
            let start_time = Instant::now();
            let sleep = time::sleep(timeout);
            tokio::pin!(sleep);

            loop {
                tokio::select! {
                    _ = &mut sleep => {
                        trace!("[{}] interactive transaction timed out", server.id.to_string());
                        let _ = server.rollback(true).await;
                        break;
                    }
                    msg = server.receive.recv() => {
                        if let Some(op) = msg {
                            let run_state = server.process_msg(op).await;

                            if run_state == RunState::Finished {
                                break
                            }
                        }
                    }
                }
            }

            trace!("[{}] completed with {}", server.id.to_string(), server.cached_tx);

            let _ = send_done
                .send((
                    server.id.clone(),
                    server.cached_tx.to_closed(start_time, server.timeout),
                ))
                .await;

            trace!("[{}] has stopped with {}", server.id.to_string(), server.cached_tx);
        })
        .instrument(span)
        .with_subscriber(dispatcher),
    );

    client
}

/// Spawn the client list clear actor
/// It waits for messages from completed ITXServers and removes
/// the ITXClient from the clients hashmap

/* A future improvement to this would be to change this to keep a queue of
   clients to remove from the list and then periodically remove them. This
   would be a nice optimization because we would take less write locks on the
   hashmap.

   The downside to consider is that we can introduce a race condition where the
   ITXServer has stopped running but the client hasn't been removed from the hashmap
   yet. When the client tries to send a message to the ITXServer there will be a
   send error. This isn't a huge obstacle but something to handle correctly.
   And example implementation for this would be:

   ```
        let mut queue: Vec<TxId> = Vec::new();

        let sleep_duration = Duration::from_millis(100);
        let clear_sleeper = time::sleep(sleep_duration);
        tokio::pin!(clear_sleeper);

        loop {
            tokio::select! {
                _ = &mut clear_sleeper => {
                    let mut list = clients.write().await;
                    for id in queue.drain(..) {
                        trace!("removing {} from client list", id);
                        list.remove(&id);
                    }
                    clear_sleeper.as_mut().reset(Instant::now() + sleep_duration);
                }
                msg = rx.recv() => {
                    if let Some(id) = msg {
                        queue.push(id);
                    }
                }
            }
        }
   ```
*/
pub fn spawn_client_list_clear_actor(
    clients: Arc<RwLock<HashMap<TxId, ITXClient>>>,
    closed_txs: Arc<RwLock<lru::LruCache<TxId, Option<ClosedTx>>>>,
    mut rx: Receiver<(TxId, Option<ClosedTx>)>,
) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        loop {
            if let Some((id, closed_tx)) = rx.recv().await {
                trace!("removing {} from client list", id);

                let mut clients_guard = clients.write().await;
                clients_guard.remove(&id);
                drop(clients_guard);

                closed_txs.write().await.put(id, closed_tx);
            }
        }
    })
}
