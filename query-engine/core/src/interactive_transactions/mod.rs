use crate::CoreError;
use connector::Transaction;
use crosstarget_utils::time::ElapsedTimeCounter;
use derive_more::Display;
use serde::Deserialize;
use tokio::time::Duration;

mod error;
mod manager;
mod transaction;

pub use error::*;

pub(crate) use manager::*;
pub(crate) use transaction::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Display)]
#[display(fmt = "{}", "0")]
pub struct TxId(String);

impl TxId {
    pub fn new() -> Self {
        #[allow(deprecated)]
        Self(cuid::cuid().unwrap())
    }
}

impl<T> From<T> for TxId
where
    T: Into<String>,
{
    fn from(s: T) -> Self {
        const MINIMUM_TX_ID_LENGTH: usize = 24;

        let contents = s.into();
        // This postcondition is to ensure that the TxId is long enough as to be able to derive
        // a TraceId from it. See `TxTraceExt` trait for more details.
        assert!(
            contents.len() >= MINIMUM_TX_ID_LENGTH,
            "minimum length for a TxId ({}) is {}, but was {}",
            contents,
            MINIMUM_TX_ID_LENGTH,
            contents.len()
        );
        Self(contents)
    }
}

pub enum CachedTx {
    Open(Box<dyn Transaction>),
    Committed,
    RolledBack,
    Expired {
        start_time: ElapsedTimeCounter,
        timeout: Duration,
    },
}

impl CachedTx {
    /// Requires this cached TX to be `Open`, else an error will be raised that it is no longer valid.
    pub(crate) fn as_open(&mut self, from_operation: &str) -> crate::Result<&mut Box<dyn Transaction>> {
        match self {
            CachedTx::Open(tx) => Ok(tx),
            tx => Err(CoreError::from(TransactionError::Closed {
                reason: tx.to_closed().unwrap().error_message_for(from_operation),
            })),
        }
    }

    pub(crate) fn to_closed(&self) -> Option<ClosedTx> {
        match self {
            CachedTx::Open(_) => None,
            CachedTx::Committed => Some(ClosedTx::Committed),
            CachedTx::RolledBack => Some(ClosedTx::RolledBack),
            CachedTx::Expired { start_time, timeout } => Some(ClosedTx::Expired {
                start_time: *start_time,
                timeout: *timeout,
            }),
        }
    }
}

pub(crate) enum ClosedTx {
    Committed,
    RolledBack,
    Expired {
        start_time: ElapsedTimeCounter,
        timeout: Duration,
    },
}

impl ClosedTx {
    pub fn error_message_for(&self, operation: &str) -> String {
        match self {
            ClosedTx::Committed => {
                format!("A {operation} cannot be executed on a committed transaction")
            }
            ClosedTx::RolledBack => {
                format!("A {operation} cannot be executed on a transaction that was rolled back")
            }
            ClosedTx::Expired { start_time, timeout } => {
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
