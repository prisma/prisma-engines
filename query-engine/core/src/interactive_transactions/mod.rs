use crate::CoreError;
use connector::{Connection, ConnectionLike, Transaction};
use std::{collections::HashMap, fmt::Display};
use tokio::{
    task::JoinHandle,
    time::{Duration, Instant},
};

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

const MINIMUM_TX_ID_LENGTH: usize = 24;

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
        let contents = s.into();
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
        write!(f, "{}", self.0)
    }
}

impl From<TxId> for opentelemetry::Context {
    // This is a bit of a hack, but it's the only way to have a default trace span for a whole
    // transaction when no traceparent is propagated from the client.
    //
    // This is done  so we can capture traces happening accross the different queries in a
    // transaction. Otherwise, if a traceparent is not propagated from the client, each query in
    // the transaction will run within a span that has already been generated at the begining of the
    // transaction, and held active in the actor in charge of running the queries. Thus, making
    // impossible to capture traces happening in the individual queries, as they won't be aware of
    // the transaction they are part of.
    //
    // By generating this "fake" traceparent based on the transaction id, we can have a common
    // trace_id for all transaction operations.
    fn from(id: TxId) -> Self {
        let extractor: HashMap<String, String> =
            HashMap::from_iter(vec![("traceparent".to_string(), id.as_traceparent())]);
        opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
    }
}

impl TxId {
    pub fn as_traceparent(&self) -> String {
        let trace_id = opentelemetry::trace::TraceId::from(self.clone());
        format!("00-{}-0000000000000001-01", trace_id)
    }
}

impl From<TxId> for opentelemetry::trace::TraceId {
    // in order to convert a TxId (a 48 bytes cuid) into a TraceId (16 bytes), we remove the first byte,
    // (always 'c') and get the next 16 bytes, which are random enough to be used as a trace id.
    // this is a typical cuid: "c-lct0q6ma-0004-rb04-h6en1roa"
    //
    // - first letter is always the same
    // - next 7-8 byte are random a timestamp. There's more entropy in the least significative bytes
    // - next 4 bytes are a counter since the server started
    // - next 4 bytes are a system fingerprint, invariant for the same server instance
    // - least significative 8 bytes. Totally random.
    //
    // We want the most entropic slice of 16 bytes that's deterministicly determined
    fn from(id: TxId) -> Self {
        let mut buffer = [0; 16];
        let tx_id_bytes = id.0.as_bytes();
        let len = tx_id_bytes.len();

        // bytes [len-20  to len-12): least significative 4 bytes of the timestamp + 4 bytes counter
        for (i, source_idx) in (len - 20..len - 12).enumerate() {
            buffer[i] = tx_id_bytes[source_idx];
        }
        // bytes [len-8 to len):  the random blocks
        for (i, source_idx) in (len - 8..len).enumerate() {
            buffer[i + 8] = tx_id_bytes[source_idx];
        }

        opentelemetry::trace::TraceId::from_bytes(buffer)
    }
}

pub enum CachedTx {
    Open(OpenTx),
    Committed,
    RolledBack,
    Expired,
}

impl Display for CachedTx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CachedTx::Open(_) => write!(f, "Open"),
            CachedTx::Committed => write!(f, "Committed"),
            CachedTx::RolledBack => write!(f, "Rolled back"),
            CachedTx::Expired => write!(f, "Expired"),
        }
    }
}

impl CachedTx {
    /// Requires this cached TX to be `Open`, else an error will be raised that it is no longer valid.
    pub fn as_open(&mut self) -> crate::Result<&mut OpenTx> {
        if let Self::Open(ref mut otx) = self {
            Ok(otx)
        } else {
            let reason = format!("Transaction is no longer valid. Last state: '{}'", self);
            Err(CoreError::from(TransactionError::Closed { reason }))
        }
    }

    pub fn to_closed(&self, start_time: Instant, timeout: Duration) -> Option<ClosedTx> {
        match self {
            CachedTx::Open(_) => None,
            CachedTx::Committed => Some(ClosedTx::Committed),
            CachedTx::RolledBack => Some(ClosedTx::RolledBack),
            CachedTx::Expired => Some(ClosedTx::Expired { start_time, timeout }),
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

pub enum ClosedTx {
    Committed,
    RolledBack,
    Expired { start_time: Instant, timeout: Duration },
}

// tests for txid into traits
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_txid_into_traceid() {
        let fixture = vec![
            ("clct0q6ma0000rb04768tiqbj", "71366d6130303030373638746971626a"),
            // counter changed, trace id changed:
            ("clct0q6ma0002rb04cpa6zkmx", "71366d6130303032637061367a6b6d78"),
            // fingerprint changed, trace id did not change, as that chunk is ignored:
            ("clct0q6ma00020000cpa6zkmx", "71366d6130303032637061367a6b6d78"),
            // first 5 bytes changed, trace id did not change, as that chunk is ignored:
            ("00000q6ma00020000cpa6zkmx", "71366d6130303032637061367a6b6d78"),
            // 6 th byte changed, trace id changed, as that chunk is part of the lsb of the timestamp
            ("0000006ma00020000cpa6zkmx", "30366d6130303032637061367a6b6d78"),
        ];

        for (txid, expected_trace_id) in fixture {
            let txid = TxId(txid.to_string());
            let trace_id: opentelemetry::trace::TraceId = txid.into();
            assert_eq!(trace_id.to_string(), expected_trace_id);
        }
    }
}
