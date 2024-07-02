#![allow(unsafe_code)]

use std::pin::Pin;

use super::CachedTx;
use crate::{
    execute_many_operations, execute_single_operation, get_current_dispatcher, ClosedTx, Operation, ResponseData,
    TransactionError, TxId,
};
use connector::{Connection, Transaction};
use crosstarget_utils::time::ElapsedTimeCounter;
use schema::QuerySchemaRef;
use tokio::time::Duration;
use tracing::{instrument::WithSubscriber, Dispatch, Span};
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

    pub async fn start_transaction(&mut self, isolation_level: Option<String>) -> crate::Result<()> {
        // SAFETY: We do not move out of `self.conn`.
        let conn_mut: &mut (dyn Connection + Send + Sync) = unsafe { self.conn.as_mut().get_unchecked_mut() };

        // This creates a transaction, which borrows from the connection.
        let tx_borrowed_from_conn: Box<dyn Transaction> = conn_mut.start_transaction(isolation_level).await?;

        // SAFETY: This transmute only erases the lifetime from `conn_mut`. Normally, borrow checker
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

    pub(crate) fn to_closed(&self, start_time: ElapsedTimeCounter, timeout: Duration) -> Option<ClosedTx> {
        self.cached_tx.to_closed(start_time, timeout)
    }
}

pub struct InteractiveTransaction {
    id: TxId,
    state: TransactionState,
    start_time: ElapsedTimeCounter,
    timeout: Duration,
    query_schema: QuerySchemaRef,
    span: Span,
    dispatcher: Dispatch,
}

macro_rules! tx_timeout {
    ($self:expr, $fut:expr) => {
        if let Some(remaining_time) = $self.timeout.checked_sub($self.start_time.elapsed_time()) {
            tokio::select! {
                _ = crosstarget_utils::time::sleep(remaining_time) => {
                    $self.rollback(true).await?;
                    Err(TransactionError::Closed {
                        reason: "Could not perform operation".to_string(),
                    }.into())
                }
                result = $fut => {
                    result
                }
            }
        } else {
            $self.rollback(true).await?;
            Err(TransactionError::Closed {
                reason: "Could not perform operation".to_string(),
            }
            .into())
        }
    };
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

        let span = Span::current();
        span.record("itx_id", id.to_string());

        Ok(Self {
            id,
            state,
            start_time: ElapsedTimeCounter::start(),
            timeout,
            query_schema,
            span,
            dispatcher: get_current_dispatcher(),
        })
    }

    pub(crate) async fn execute_single(
        &mut self,
        operation: &Operation,
        traceparent: Option<String>,
    ) -> crate::Result<ResponseData> {
        let span = info_span!(parent: &self.span, "prisma:engine:itx_execute_single", user_facing = true);
        #[cfg(feature = "metrics")]
        set_span_link_from_traceparent(&span, traceparent.clone());
        let conn = self.state.as_open()?;

        tx_timeout!(
            self,
            execute_single_operation(
                self.query_schema.clone(),
                conn.as_connection_like(),
                operation,
                traceparent,
            )
            .instrument(span)
            .with_subscriber(self.dispatcher.clone())
        )
    }

    pub(crate) async fn execute_batch(
        &mut self,
        operations: &[Operation],
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        tx_timeout!(self, async {
            let span = info_span!(parent: &self.span, "prisma:engine:itx_execute_batch", user_facing = true);
            #[cfg(feature = "metrics")]
            set_span_link_from_traceparent(&span, traceparent.clone());
            let conn = self.state.as_open()?;
            execute_many_operations(
                self.query_schema.clone(),
                conn.as_connection_like(),
                operations,
                traceparent,
            )
            .instrument(span)
            .with_subscriber(self.dispatcher.clone())
            .await
        })
    }

    pub(crate) async fn commit(&mut self) -> crate::Result<()> {
        tx_timeout!(self, async {
            if let Ok(open_tx) = self.state.as_open() {
                let span = info_span!(parent: &self.span, "prisma:engine:itx_commit", user_facing = true);
                open_tx
                    .commit()
                    .instrument(span)
                    .with_subscriber(self.dispatcher.clone())
                    .await?;
                self.state.set_committed();
            }
            Ok(())
        })
    }

    pub(crate) async fn rollback(&mut self, was_timeout: bool) -> crate::Result<()> {
        debug!("[{}] rolling back, was timed out = {was_timeout}", self.name());
        if let Ok(open_tx) = self.state.as_open() {
            let span = info_span!(parent: &self.span, "prisma:engine:itx_rollback", user_facing = true);
            open_tx
                .rollback()
                .instrument(span)
                .with_subscriber(self.dispatcher.clone())
                .await?;
            if was_timeout {
                self.state.set_expired();
            } else {
                self.state.set_rolled_back();
            }
        }

        Ok(())
    }

    pub(crate) fn to_closed(&self) -> Option<ClosedTx> {
        self.state.to_closed(self.start_time, self.timeout)
    }

    pub(crate) fn name(&self) -> String {
        format!("itx-{:?}", self.id.to_string())
    }
}
