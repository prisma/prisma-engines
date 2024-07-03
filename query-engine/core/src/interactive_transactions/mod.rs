use crate::CoreError;
use connector::Transaction;
use crosstarget_utils::time::ElapsedTimeCounter;
use serde::Deserialize;
use std::fmt::Display;
use tokio::time::Duration;

mod error;
mod manager;
mod transaction;

pub use error::*;

pub(crate) use manager::*;
pub(crate) use transaction::*;

/// How Interactive Transactions work
/// The Interactive Transactions (iTx) follow an actor model design. Where each iTx is created in its own process.
/// When a prisma client requests to start a new transaction, the Transaction Actor Manager spawns a new ITXServer. The ITXServer runs in its own
/// process and waits for messages to arrive via its receive channel to process.
/// The Transaction Actor Manager will also create an ITXClient and add it to hashmap managed by an RwLock. The ITXClient is the only way to communicate
/// with the ITXServer.

/// Once Prisma Client receives the iTx Id it can perform database operations using that iTx id. When an operation request is received by the
/// TransactionActorManager, it looks for the client in the hashmap and passes the operation to the client. The ITXClient sends a message to the
/// ITXServer and waits for a response. The ITXServer will then perform the operation and return the result. The ITXServer will perform one
/// operation at a time. All other operations will sit in the message queue waiting to be processed.
///
/// The ITXServer will handle all messages until:
/// - It transitions state, e.g "rollback" or "commit"
/// - It exceeds its timeout, in which case the iTx is rolledback and the connection to the database is closed.

/// Once the ITXServer is done handling messages from the iTx Client, it sends a last message to the Background Client list Actor to say that it is completed and then shuts down.
/// The Background Client list Actor removes the client from the list of active clients and keeps in cache the iTx id of the closed transaction.

/// We keep a list of closed transactions so that if any further messages are received for this iTx id,
/// the TransactionActorManager can reply with a helpful error message which explains that no operation can be performed on a closed transaction
/// rather than an error message stating that the transaction does not exist.

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
pub struct TxId(String);

const MINIMUM_TX_ID_LENGTH: usize = 24;

impl Default for TxId {
    fn default() -> Self {
        #[allow(deprecated)]
        Self(cuid::cuid().unwrap())
    }
}

impl<T> From<T> for TxId
where
    T: Into<String>,
{
    fn from(s: T) -> Self {
        let contents = s.into();
        // This postcondition is to ensure that the TxId is long enough as to be able to derive
        // a TraceId from it.
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

impl Display for TxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
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
            tx @ _ => Err(CoreError::from(TransactionError::Closed {
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
