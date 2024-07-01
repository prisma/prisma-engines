#![allow(unsafe_code)]

use std::pin::Pin;

use super::CachedTx;
use crate::{execute_many_operations, execute_single_operation, Operation, ResponseData, TxId};
use connector::{Connection, Transaction};
use schema::QuerySchemaRef;
use tokio::time::Duration;
use tracing_futures::Instrument;

#[cfg(feature = "metrics")]
use crate::telemetry::helpers::set_span_link_from_traceparent;

struct TransactionState {
    // Note: field order is important here because fields are dropped in the declaration order.
    // First, we drop the `cached_tx`, which may reference `conn`. Only after that we drop `conn`.
    cached_tx: CachedTx,
    conn: Pin<Box<dyn Connection + Send + Sync>>,
}

impl TransactionState {
    pub fn new(conn: Box<dyn Connection + Send + Sync>) -> Self {
        Self {
            conn: Box::into_pin(conn),
            cached_tx: CachedTx::Expired,
        }
    }

    pub async fn start_transaction<'conn>(&'conn mut self, isolation_level: Option<String>) -> crate::Result<()> {
        // SAFETY: We do not move out of `self.conn`.
        let conn_mut: &'conn mut (dyn Connection + Send + Sync) = unsafe { self.conn.as_mut().get_unchecked_mut() };

        // This creates a transaction, which borrows from the connection.
        let tx_borrowed_from_conn: Box<dyn Transaction + 'conn> = conn_mut.start_transaction(isolation_level).await?;

        // SAFETY: This transmute only erases the 'conn lifetime. Normally, borrow checker
        // guarantees that the borrowed value is not dropped. In this case, we guarantee ourselves
        // through the use of `Pin` on the connection.
        let tx_with_erased_lifetime: Box<dyn Transaction + 'static> =
            unsafe { std::mem::transmute(tx_borrowed_from_conn) };

        self.cached_tx = CachedTx::Open(tx_with_erased_lifetime);
        Ok(())
    }

    pub fn as_open(&mut self) -> crate::Result<&mut Box<dyn Transaction>> {
        self.cached_tx.as_open()
    }

    pub fn set_committed(&mut self) {
        self.cached_tx = CachedTx::Committed;
    }

    pub fn set_expired(&mut self) {
        self.cached_tx = CachedTx::Expired;
    }

    pub fn set_rolled_back(&mut self) {
        self.cached_tx = CachedTx::RolledBack;
    }
}

pub struct InteractiveTransaction {
    id: TxId,
    state: TransactionState,
    timeout: Duration,
    query_schema: QuerySchemaRef,
}

impl InteractiveTransaction {
    pub async fn new(
        id: TxId,
        conn: Box<dyn Connection + Send + Sync>,
        timeout: Duration,
        query_schema: QuerySchemaRef,
        isolation_level: Option<String>,
    ) -> crate::Result<Self> {
        let mut state = TransactionState::new(conn);
        state.start_transaction(isolation_level).await?;

        Ok(Self {
            id,
            state,
            timeout,
            query_schema,
        })
    }

    pub(crate) async fn execute_single(
        &mut self,
        operation: &Operation,
        traceparent: Option<String>,
    ) -> crate::Result<ResponseData> {
        let span = info_span!("prisma:engine:itx_query_builder", user_facing = true);

        #[cfg(feature = "metrics")]
        set_span_link_from_traceparent(&span, traceparent.clone());

        let conn = self.state.as_open()?;
        execute_single_operation(
            self.query_schema.clone(),
            conn.as_connection_like(),
            operation,
            traceparent,
        )
        .instrument(span)
        .await
    }

    pub(crate) async fn execute_batch(
        &mut self,
        operations: &[Operation],
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        let span = info_span!("prisma:engine:itx_execute", user_facing = true);

        let conn = self.state.as_open()?;
        execute_many_operations(
            self.query_schema.clone(),
            conn.as_connection_like(),
            operations,
            traceparent,
        )
        .instrument(span)
        .await
    }

    pub(crate) async fn commit(&mut self) -> crate::Result<()> {
        if let Ok(open_tx) = self.state.as_open() {
            trace!("[{}] committing.", self.id.to_string());
            open_tx.commit().await?;
            self.state.set_committed();
        }

        Ok(())
    }

    pub(crate) async fn rollback(&mut self, was_timeout: bool) -> crate::Result<()> {
        debug!("[{}] rolling back, was timed out = {was_timeout}", self.name());
        if let Ok(open_tx) = self.state.as_open() {
            open_tx.rollback().await?;
            if was_timeout {
                trace!("[{}] Expired Rolling back", self.id.to_string());
                self.state.set_expired();
            } else {
                self.state.set_rolled_back();
                trace!("[{}] Rolling back", self.id.to_string());
            }
        }

        Ok(())
    }

    pub(crate) fn name(&self) -> String {
        format!("itx-{:?}", self.id.to_string())
    }
}

// #[allow(clippy::too_many_arguments)]
// pub(crate) async fn spawn_itx_actor(
//     query_schema: QuerySchemaRef,
//     tx_id: TxId,
//     mut conn: Box<dyn Connection + Send + Sync>,
//     isolation_level: Option<String>,
//     timeout: Duration,
//     channel_size: usize,
//     send_done: Sender<(TxId, Option<ClosedTx>)>,
//     engine_protocol: EngineProtocol,
// ) -> crate::Result<ITXClient> {
//     let span = Span::current();
//     let tx_id_str = tx_id.to_string();
//     span.record("itx_id", tx_id_str.as_str());
//     let dispatcher = crate::get_current_dispatcher();

//     let (tx_to_server, rx_from_client) = channel::<TxOpRequest>(channel_size);
//     let client = ITXClient {
//         send: tx_to_server,
//         tx_id: tx_id.clone(),
//     };
//     let (open_transaction_send, open_transaction_rcv) = oneshot::channel();

//     spawn(
//         crate::executor::with_request_context(engine_protocol, async move {
//             // We match on the result in order to send the error to the parent task and abort this
//             // task, on error. This is a separate task (actor), not a function where we can just bubble up the
//             // result.
//             let c_tx = match conn.start_transaction(isolation_level).await {
//                 Ok(c_tx) => {
//                     open_transaction_send.send(Ok(())).unwrap();
//                     c_tx
//                 }
//                 Err(err) => {
//                     open_transaction_send.send(Err(err)).unwrap();
//                     return;
//                 }
//             };

//             let mut server = ITXServer::new(
//                 tx_id.clone(),
//                 CachedTx::Open(c_tx),
//                 timeout,
//                 rx_from_client,
//                 query_schema,
//             );

//             let start_time = ElapsedTimeCounter::start();
//             let sleep = crosstarget_utils::time::sleep(timeout);
//             tokio::pin!(sleep);

//             loop {
//                 tokio::select! {
//                     _ = &mut sleep => {
//                         trace!("[{}] interactive transaction timed out", server.id.to_string());
//                         let _ = server.rollback(true).await;
//                         break;
//                     }
//                     msg = server.receive.recv() => {
//                         if let Some(op) = msg {
//                             let run_state = server.process_msg(op).await;

//                             if run_state == RunState::Finished {
//                                 break
//                             }
//                         } else {
//                             break;
//                         }
//                     }
//                 }
//             }

//             trace!("[{}] completed with {}", server.id.to_string(), server.cached_tx);

//             let _ = send_done
//                 .send((
//                     server.id.clone(),
//                     server.cached_tx.to_closed(start_time, server.timeout),
//                 ))
//                 .await;

//             trace!("[{}] has stopped with {}", server.id.to_string(), server.cached_tx);
//         })
//         .instrument(span)
//         .with_subscriber(dispatcher),
//     );

//     open_transaction_rcv.await.unwrap()?;

//     Ok(client)
// }
