use crate::CoreError;
use connector::Transaction;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tokio::time::{Duration, Instant};

mod actor_manager;
mod actors;
mod error;
mod messages;

pub use error::*;

pub(crate) use actor_manager::*;
pub(crate) use actors::*;
pub(crate) use messages::*;

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

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
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

pub enum CachedTx<'a> {
    Open(Box<dyn Transaction + 'a>),
    Committed,
    RolledBack,
    Expired,
}

impl Display for CachedTx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CachedTx::Open(_) => f.write_str("Open"),
            CachedTx::Committed => f.write_str("Committed"),
            CachedTx::RolledBack => f.write_str("Rolled back"),
            CachedTx::Expired => f.write_str("Expired"),
        }
    }
}

impl<'a> CachedTx<'a> {
    /// Requires this cached TX to be `Open`, else an error will be raised that it is no longer valid.
    pub(crate) fn as_open(&mut self) -> crate::Result<&mut Box<dyn Transaction + 'a>> {
        if let Self::Open(ref mut otx) = self {
            Ok(otx)
        } else {
            let reason = format!("Transaction is no longer valid. Last state: '{self}'");
            Err(CoreError::from(TransactionError::Closed { reason }))
        }
    }

    pub(crate) fn to_closed(&self, start_time: Instant, timeout: Duration) -> Option<ClosedTx> {
        match self {
            CachedTx::Open(_) => None,
            CachedTx::Committed => Some(ClosedTx::Committed),
            CachedTx::RolledBack => Some(ClosedTx::RolledBack),
            CachedTx::Expired => Some(ClosedTx::Expired { start_time, timeout }),
        }
    }
}

pub(crate) enum ClosedTx {
    Committed,
    RolledBack,
    Expired { start_time: Instant, timeout: Duration },
}
