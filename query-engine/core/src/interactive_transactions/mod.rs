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
/// The ITXServer will handle all messages until it transitions state, e.g "rollback" or "commit".

/// After that the ITXServer will move into the cache eviction state. In this state, the connection is closed, and any messages it receives, it will
/// will reply with its last state. i.e committed, rollbacked or timeout. The eviction state is there so that if a prisma wants to
// perform an action on a iTx that has completed it will get a better message rather than the error message that this transaction doesn't exist

/// Once the eviction timeout is exceeded, the ITXServer will send a message to the Background Client list Actor to say that it is completed,
/// and the ITXServer will end. The Background Client list Actor removes the client from the list of clients that are active.

/// During the time the ITXServer is active there is a timer running and if that timeout is exceeded, the
/// transaction is rolledback and the connection to the database is closed. The ITXServer will then move into the eviction state.
///

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
