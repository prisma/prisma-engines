use crate::CoreError;
use connector::{Connection, ConnectionLike, Transaction};
use std::fmt::Display;
use tokio::task::JoinHandle;

mod actor_manager;
mod actors;
mod error;
mod messages;

pub use actor_manager::*;
pub use actors::*;
pub use error::*;
pub use messages::*;

/// How Interactive Transactions work
/// The Interactive Transactions (iTx) follow an actor model design. Where each iTx is created in its own process.
/// When a prisma client requests to start a new transaction, the Transaction Actor Manager spawns a new ITXServer. The ITXServer runs in its own
/// process and waits for messages to arrive via its receive channel to process.
/// The Transaction Actor Manager will also create an ITXClient and add it to hashmap managed by an RwLock. The ITXClient is the only way to communicate
/// with the ITXServer.

/// Once the prisma client receives the iTx Id it can perform database operations using that iTx id. When an operation request is received by the
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
    pub async fn start(mut conn: Box<dyn Connection>, isolation_level: Option<String>) -> crate::Result<Self> {
        // Forces static lifetime for the transaction, disabling the lifetime checks for `tx`.
        // Why is this okay? We store the connection the tx depends on with its lifetime next to
        // the tx in the struct. Neither the connection nor the tx are moved out of this struct.
        // The `OpenTx` struct is dropped as a unit.
        let transaction: Box<dyn Transaction + '_> = conn.start_transaction(isolation_level).await?;
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
