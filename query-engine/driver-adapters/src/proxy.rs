use crate::send_future::UnsafeFuture;
use crate::types::JsConnectionInfo;
pub use crate::types::{JSResultSet, Query, TransactionOptions};
use crate::{
    from_js_value, get_named_property, get_optional_named_property, to_rust_str, AdapterMethod, JsObject, JsResult,
    JsString, JsTransaction,
};

use futures::Future;
use metrics::increment_gauge;
use std::sync::atomic::{AtomicBool, Ordering};

/// Proxy is a struct wrapping a javascript object that exhibits basic primitives for
/// querying and executing SQL (i.e. a client connector). The Proxy uses Napi/Wasm's JsFunction
/// to invoke the code within the node runtime that implements the client connector.
pub(crate) struct CommonProxy {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: AdapterMethod<Query, JSResultSet>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: AdapterMethod<Query, u32>,

    /// Return the provider for this driver.
    pub(crate) provider: String,
}

/// This is a JS proxy for accessing the methods specific to top level
/// JS driver objects
pub(crate) struct DriverProxy {
    start_transaction: AdapterMethod<(), JsTransaction>,
    get_connection_info: Option<AdapterMethod<(), JsConnectionInfo>>,
}

/// This a JS proxy for accessing the methods, specific
/// to JS transaction objects
pub(crate) struct TransactionProxy {
    /// transaction options
    options: TransactionOptions,

    /// begin transaction
    pub begin: AdapterMethod<(), ()>,

    /// commit transaction
    commit: AdapterMethod<(), ()>,

    /// rollback transaction
    rollback: AdapterMethod<(), ()>,

    /// whether the transaction has already been committed or rolled back
    closed: AtomicBool,
}

impl CommonProxy {
    pub fn new(object: &JsObject) -> JsResult<Self> {
        let provider: JsString = get_named_property(object, "provider")?;

        Ok(Self {
            query_raw: get_named_property(object, "queryRaw")?,
            execute_raw: get_named_property(object, "executeRaw")?,
            provider: to_rust_str(provider)?,
        })
    }

    pub async fn query_raw(&self, params: Query) -> quaint::Result<JSResultSet> {
        self.query_raw.call_as_async(params).await
    }

    pub async fn execute_raw(&self, params: Query) -> quaint::Result<u32> {
        self.execute_raw.call_as_async(params).await
    }
}

impl DriverProxy {
    pub fn new(object: &JsObject) -> JsResult<Self> {
        Ok(Self {
            start_transaction: get_named_property(object, "startTransaction")?,
            get_connection_info: get_optional_named_property(object, "getConnectionInfo")?,
        })
    }

    pub async fn get_connection_info(&self) -> quaint::Result<JsConnectionInfo> {
        UnsafeFuture(async move {
            if let Some(fn_) = &self.get_connection_info {
                fn_.call_as_sync(()).await
            } else {
                Ok(JsConnectionInfo::default())
            }
        })
        .await
    }

    async fn start_transaction_inner(&self) -> quaint::Result<Box<JsTransaction>> {
        let tx = self.start_transaction.call_as_async(()).await?;

        // Decrement for this gauge is done in JsTransaction::commit/JsTransaction::rollback
        // Previously, it was done in JsTransaction::new, similar to the native Transaction.
        // However, correct Dispatcher is lost there and increment does not register, so we moved
        // it here instead.
        increment_gauge!("prisma_client_queries_active", 1.0);
        Ok(Box::new(tx))
    }

    pub fn start_transaction(&self) -> UnsafeFuture<impl Future<Output = quaint::Result<Box<JsTransaction>>> + '_> {
        UnsafeFuture(self.start_transaction_inner())
    }
}

impl TransactionProxy {
    pub fn new(js_transaction: &JsObject) -> JsResult<Self> {
        let commit = get_named_property(js_transaction, "commit")?;
        let begin = get_named_property(js_transaction, "begin")?;
        let rollback = get_named_property(js_transaction, "rollback")?;
        let options = get_named_property(js_transaction, "options")?;
        let options = from_js_value::<TransactionOptions>(options);

        Ok(Self {
            begin,
            commit,
            rollback,
            options,
            closed: AtomicBool::new(false),
        })
    }

    pub fn options(&self) -> &TransactionOptions {
        &self.options
    }

    pub fn begin(&self) -> UnsafeFuture<impl Future<Output = quaint::Result<()>> + '_> {
        self.closed.store(true, Ordering::Relaxed);
        UnsafeFuture(self.begin.call_as_async(()))
    }

    /// Commits the transaction via the driver adapter.
    ///
    /// ## Cancellation safety
    ///
    /// The future is cancellation-safe as long as the underlying Node-API call
    /// is cancellation-safe and no new await points are introduced between storing true in
    /// [`TransactionProxy::closed`] and calling the underlying JS function.
    ///
    /// - If `commit` is called but never polled or awaited, it's a no-op, the transaction won't be
    ///   committed and [`TransactionProxy::closed`] will not be changed.
    ///
    /// - If it is polled at least once, `true` will be stored in [`TransactionProxy::closed`] and
    ///   the underlying FFI call will be delivered to JavaScript side in lockstep, so the destructor
    ///   will not attempt rolling the transaction back even if the `commit` future was dropped while
    ///   waiting on the JavaScript call to complete and deliver response.
    pub fn commit(&self) -> UnsafeFuture<impl Future<Output = quaint::Result<()>> + '_> {
        self.closed.store(true, Ordering::Relaxed);
        UnsafeFuture(self.commit.call_as_async(()))
    }

    /// Rolls back the transaction via the driver adapter.
    ///
    /// ## Cancellation safety
    ///
    /// The future is cancellation-safe as long as the underlying Node-API call
    /// is cancellation-safe and no new await points are introduced between storing true in
    /// [`TransactionProxy::closed`] and calling the underlying JS function.
    ///
    /// - If `rollback` is called but never polled or awaited, it's a no-op, the transaction won't be
    ///   rolled back yet and [`TransactionProxy::closed`] will not be changed.
    ///
    /// - If it is polled at least once, `true` will be stored in [`TransactionProxy::closed`] and
    ///   the underlying FFI call will be delivered to JavaScript side in lockstep, so the destructor
    ///   will not attempt rolling back again even if the `rollback` future was dropped while waiting
    ///   on the JavaScript call to complete and deliver response.
    pub fn rollback(&self) -> UnsafeFuture<impl Future<Output = quaint::Result<()>> + '_> {
        self.closed.store(true, Ordering::Relaxed);
        UnsafeFuture(self.rollback.call_as_async(()))
    }
}

impl Drop for TransactionProxy {
    fn drop(&mut self) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }

        self.rollback.call_non_blocking(());
    }
}

macro_rules! impl_send_sync_on_wasm {
    ($struct:ident) => {
        #[cfg(target_arch = "wasm32")]
        unsafe impl Send for $struct {}
        #[cfg(target_arch = "wasm32")]
        unsafe impl Sync for $struct {}
    };
}

// Assume the proxy object will not be sent to service workers, we can unsafe impl Send + Sync.
impl_send_sync_on_wasm!(TransactionProxy);
impl_send_sync_on_wasm!(DriverProxy);
impl_send_sync_on_wasm!(CommonProxy);
impl_send_sync_on_wasm!(JsTransaction);
