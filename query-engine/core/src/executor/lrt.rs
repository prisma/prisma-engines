use connector::{Connection, ConnectionLike, Transaction};
use dashmap::{mapref::one::RefMut, DashMap};
use std::{fmt::Display, sync::Arc};
use thiserror::Error;
use tokio::{
    task::{self, JoinHandle},
    time::{self, Duration},
};

use crate::CoreError;

static CACHE_EVICTION_SECS: u64 = 3;

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("Unable to start a transaction in the given time.")]
    AcquisitionTimeout,

    #[error("Attempted to start a transaction inside of a transaction.")]
    AlreadyStarted,

    #[error("Transaction not found.")]
    NotFound,

    #[error("Transaction already closed: {reason}.")]
    Closed { reason: String },
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TxId(String);

impl TxId {
    pub fn new() -> Self {
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
    /// Consumes self to remove the `CachedTx` indirection to get to the underlying `OpenTx`.
    pub fn into_open(self) -> crate::Result<OpenTx> {
        if let Self::Open(otx) = self {
            Ok(otx)
        } else {
            let reason = format!("Transaction is no longer valid. Last state: '{}'.", self);
            Err(CoreError::from(TransactionError::Closed { reason }))
        }
    }

    /// Requires this cached TX to be `Open`, else an error will be raised that it is no longer valid.
    pub fn as_open(&mut self) -> crate::Result<&mut OpenTx> {
        if let Self::Open(ref mut otx) = self {
            Ok(otx)
        } else {
            let reason = format!("Transaction is no longer valid. Last state: '{}'.", self);
            Err(CoreError::from(TransactionError::Closed { reason }))
        }
    }
}

#[derive(Default)]
pub(crate) struct TransactionCache {
    cache: Arc<DashMap<TxId, CachedTx>>,
}

#[allow(dead_code)]
impl TransactionCache {
    pub fn dump_keys(&self) {
        dbg!(self.cache.len());
        self.cache.iter().for_each(|r| {
            dbg!(r.key());
        });
    }
}

impl TransactionCache {
    pub async fn insert(&self, key: TxId, mut value: OpenTx, valid_for_secs: u64) {
        let cache = Arc::clone(&self.cache);
        let cache_key = key.clone();

        let timer_handle = task::spawn(async move {
            debug!("[{}] Valid for {} seconds", cache_key, valid_for_secs);
            time::sleep(Duration::from_secs(valid_for_secs)).await;
            debug!("[{}] Forced rollback triggered.", cache_key);

            if let Some(ref mut c_tx) = cache.get_mut(&cache_key) {
                if let CachedTx::Open(open_tx) = c_tx.value_mut() {
                    debug!("[{}] Rolling back.", cache_key.to_string());
                    open_tx.tx.rollback().await.unwrap();
                    debug!("[{}] Expired.", cache_key.to_string());
                }
            }

            cache.insert(cache_key.clone(), CachedTx::Expired);
            schedule_cache_eviction(cache_key, cache, CACHE_EVICTION_SECS);
        });

        value.expiration_timer = Some(timer_handle);
        self.cache.insert(key, CachedTx::Open(value));
    }

    /// Get cache entry or error with not found.
    pub fn get_or_err(&self, key: &TxId) -> crate::Result<RefMut<'_, TxId, CachedTx>> {
        Ok(self
            .cache
            .get_mut(key)
            // .map(|wat| wat.value_mut())
            .ok_or_else(|| TransactionError::NotFound)?)
    }

    /// Remove cache entry for `key` or error with not found.
    pub fn remove_or_err(&self, key: &TxId) -> crate::Result<CachedTx> {
        Ok(self
            .cache
            .remove(key)
            .map(|(_, c_tx)| c_tx)
            .ok_or_else(|| TransactionError::NotFound)?)
    }

    /// Replaces
    pub fn finalize_tx(&self, key: TxId, with: CachedTx) {
        self.cache.insert(key.clone(), with);
        schedule_cache_eviction(key, Arc::clone(&self.cache), CACHE_EVICTION_SECS)
    }
}

pub struct OpenTx {
    pub conn: Box<dyn Connection>,
    pub tx: Box<dyn Transaction + 'static>,
    pub expiration_timer: Option<JoinHandle<()>>,
}

impl OpenTx {
    pub async fn start(mut conn: Box<dyn Connection>) -> crate::Result<Self> {
        // Forces static lifetime for the transaction, effectively disabling the lifetime checks.
        // Requires to be extra careful with `tx`: The lifetime is basically manual now.
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

    /// Cancels a running expiration timer, if any.
    pub fn cancel_expiration_timer(&mut self) {
        if let Some(timer) = self.expiration_timer.take() {
            timer.abort();
        }
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

/// Fire-and-forget of a final cache key eviction task.
fn schedule_cache_eviction(key: TxId, cache: Arc<DashMap<TxId, CachedTx>>, secs: u64) {
    task::spawn(async move {
        time::sleep(Duration::from_secs(secs)).await;
        debug!("[{}] Evicting cache key.", key);

        if let Some(_) = cache.remove(&key) {
            debug!("[{}] Evicted.", key);
        } else {
            debug!("[{}] Already gone.", key);
        }
    });
}
