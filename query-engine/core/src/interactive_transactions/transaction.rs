#![allow(unsafe_code)]

use std::pin::Pin;

use crate::{
    execute_many_operations, execute_single_operation, CoreError, Operation, ResponseData, TransactionError, TxId,
};
use connector::{Connection, Transaction};
use crosstarget_utils::time::ElapsedTimeCounter;
use schema::QuerySchemaRef;
use tokio::time::Duration;
use tracing::Span;
use tracing_futures::Instrument;

#[cfg(feature = "metrics")]
use crate::telemetry::helpers::set_span_link_from_traceparent;

// Note: it's important to maintain the correct state of the transaction throughout execution. If
// the transaction is ever left in the `Open` state after rollback or commit operations, it means
// that the corresponding connection will never be returned to the connection pool.
enum TransactionState {
    Open {
        // Note: field order is important here because fields are dropped in the declaration order.
        // First, we drop the `tx`, which may reference `_conn`. Only after that we drop `_conn`.
        tx: Box<dyn Transaction>,
        _conn: Pin<Box<dyn Connection + Send + Sync>>,
    },
    Committed,
    RolledBack,
    Expired {
        start_time: ElapsedTimeCounter,
        timeout: Duration,
    },
}

pub enum ClosedTransaction {
    Committed,
    RolledBack,
    Expired {
        start_time: ElapsedTimeCounter,
        timeout: Duration,
    },
}

impl ClosedTransaction {
    pub fn error_message_for(&self, operation: &str) -> String {
        match self {
            ClosedTransaction::Committed => {
                format!("A {operation} cannot be executed on a committed transaction")
            }
            ClosedTransaction::RolledBack => {
                format!("A {operation} cannot be executed on a transaction that was rolled back")
            }
            ClosedTransaction::Expired { start_time, timeout } => {
                format!(
                    "A {operation} cannot be executed on an expired transaction. \
                     The timeout for this transaction was {} ms, however {} ms passed since the start \
                     of the transaction. Consider increasing the interactive transaction timeout \
                     or doing less work in the transaction",
                    timeout.as_millis(),
                    start_time.elapsed_time().as_millis(),
                )
            }
        }
    }
}

impl TransactionState {
    async fn start_transaction(
        conn: Box<dyn Connection + Send + Sync>,
        isolation_level: Option<String>,
    ) -> crate::Result<Self> {
        // Note: This method creates a self-referential struct, which is why we need unsafe. Field
        // `tx` is referencing field `conn` in the `Self::Open` variant.
        let mut conn = Box::into_pin(conn);

        // SAFETY: We do not move out of `conn`.
        let conn_mut: &mut (dyn Connection + Send + Sync) = unsafe { conn.as_mut().get_unchecked_mut() };

        // This creates a transaction, which borrows from the connection.
        let tx_borrowed_from_conn: Box<dyn Transaction> = conn_mut.start_transaction(isolation_level).await?;

        // SAFETY: This transmute only erases the lifetime from `conn_mut`. Normally, borrow checker
        // guarantees that the borrowed value is not dropped. In this case, we guarantee ourselves
        // through the use of `Pin` on the connection.
        let tx_with_erased_lifetime: Box<dyn Transaction + 'static> =
            unsafe { std::mem::transmute(tx_borrowed_from_conn) };

        Ok(Self::Open {
            tx: tx_with_erased_lifetime,
            _conn: conn,
        })
    }

    fn as_open(&mut self, from_operation: &str) -> crate::Result<&mut Box<dyn Transaction>> {
        match self {
            Self::Open { tx, .. } => Ok(tx),
            tx => Err(CoreError::from(TransactionError::Closed {
                reason: tx.as_closed().unwrap().error_message_for(from_operation),
            })),
        }
    }

    fn as_closed(&self) -> Option<ClosedTransaction> {
        match self {
            Self::Open { .. } => None,
            Self::Committed => Some(ClosedTransaction::Committed),
            Self::RolledBack => Some(ClosedTransaction::RolledBack),
            Self::Expired { start_time, timeout } => Some(ClosedTransaction::Expired {
                start_time: *start_time,
                timeout: *timeout,
            }),
        }
    }
}

pub struct InteractiveTransaction {
    id: TxId,
    state: TransactionState,
    start_time: ElapsedTimeCounter,
    timeout: Duration,
    query_schema: QuerySchemaRef,
}

/// This macro executes the future until it's ready or the transaction's timeout expires.
macro_rules! tx_timeout {
    ($self:expr, $operation:expr, $fut:expr) => {{
        let remaining_time = $self
            .timeout
            .checked_sub($self.start_time.elapsed_time())
            .unwrap_or(Duration::ZERO);
        tokio::select! {
            _ = crosstarget_utils::time::sleep(remaining_time) => {
                let _ = $self.rollback(true).await;
                Err(TransactionError::Closed {
                    reason: $self.as_closed().unwrap().error_message_for($operation),
                }.into())
            }
            result = $fut => {
                result
            }
        }
    }};
}

impl InteractiveTransaction {
    pub async fn new(
        id: TxId,
        conn: Box<dyn Connection + Send + Sync>,
        timeout: Duration,
        query_schema: QuerySchemaRef,
        isolation_level: Option<String>,
    ) -> crate::Result<Self> {
        let state = TransactionState::start_transaction(conn, isolation_level).await?;

        Span::current().record("itx_id", id.to_string());

        Ok(Self {
            id,
            state,
            start_time: ElapsedTimeCounter::start(),
            timeout,
            query_schema,
        })
    }

    pub async fn execute_single(
        &mut self,
        operation: &Operation,
        traceparent: Option<String>,
    ) -> crate::Result<ResponseData> {
        tx_timeout!(self, "query", async {
            let span = info_span!("prisma:engine:itx_execute_single", user_facing = true);
            #[cfg(feature = "metrics")]
            set_span_link_from_traceparent(&span, traceparent.clone());

            let conn = self.state.as_open("query")?;
            execute_single_operation(
                self.query_schema.clone(),
                conn.as_connection_like(),
                operation,
                traceparent,
            )
            .instrument(span)
            .await
        })
    }

    pub async fn execute_batch(
        &mut self,
        operations: &[Operation],
        traceparent: Option<String>,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>> {
        tx_timeout!(self, "batch query", async {
            let span = info_span!("prisma:engine:itx_execute_batch", user_facing = true);
            #[cfg(feature = "metrics")]
            set_span_link_from_traceparent(&span, traceparent.clone());

            let conn = self.state.as_open("batch query")?;
            execute_many_operations(
                self.query_schema.clone(),
                conn.as_connection_like(),
                operations,
                traceparent,
            )
            .instrument(span)
            .await
        })
    }

    pub async fn commit(&mut self) -> crate::Result<()> {
        tx_timeout!(self, "commit", async {
            let name = self.name();
            let open_tx = self.state.as_open("commit")?;
            let span = info_span!("prisma:engine:itx_commit", user_facing = true);

            if let Err(err) = open_tx.commit().instrument(span).await {
                debug!("transaction {name} failed to commit");
                // We don't know if the transaction was committed or not. Because of that, we cannot
                // leave it in "open" state. We attempt to rollback to get the transaction into a
                // known state.
                let _ = self.rollback(false).await;
                Err(err.into())
            } else {
                debug!("transaction {name} committed");
                self.state = TransactionState::Committed;
                Ok(())
            }
        })
    }

    pub async fn rollback(&mut self, was_timeout: bool) -> crate::Result<()> {
        let name = self.name();
        let open_tx = self.state.as_open("rollback")?;
        let span = info_span!("prisma:engine:itx_rollback", user_facing = true);

        let result = open_tx.rollback().instrument(span).await;
        if result.is_err() {
            debug!("transaction {name} failed to roll back (roll back initiated because of timeout = {was_timeout})");
        } else {
            debug!("transaction {name} rolled back (roll back initiated because of timeout = {was_timeout})");
        }

        // Ensure that the transaction isn't left in the "open" state after the rollback.
        if was_timeout {
            self.state = TransactionState::Expired {
                start_time: self.start_time,
                timeout: self.timeout,
            };
        } else {
            self.state = TransactionState::RolledBack;
        }

        result.map_err(<_>::into)
    }

    pub fn as_closed(&self) -> Option<ClosedTransaction> {
        self.state.as_closed()
    }

    pub fn name(&self) -> String {
        format!("itx-{}", self.id)
    }
}
