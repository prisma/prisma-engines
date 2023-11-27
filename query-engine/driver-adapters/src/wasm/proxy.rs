use futures::Future;
use js_sys::{Function as JsFunction, JsString};
use tsify::Tsify;

use super::{async_js_function::AsyncJsFunction, transaction::JsTransaction};
use crate::send_future::SendFuture;
pub use crate::types::{ColumnType, JSResultSet, Query, TransactionOptions};
use crate::JsObjectExtern;
use metrics::increment_gauge;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

type JsResult<T> = core::result::Result<T, JsValue>;

/// Proxy is a struct wrapping a javascript object that exhibits basic primitives for
/// querying and executing SQL (i.e. a client connector). The Proxy uses Wasm's JsFunction to
/// invoke the code within the node runtime that implements the client connector.
#[wasm_bindgen(getter_with_clone)]
pub(crate) struct CommonProxy {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: AsyncJsFunction<Query, JSResultSet>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: AsyncJsFunction<Query, u32>,

    /// Return the flavour for this driver.
    pub(crate) flavour: String,
}

/// This is a JS proxy for accessing the methods specific to top level
/// JS driver objects
#[wasm_bindgen(getter_with_clone)]
pub(crate) struct DriverProxy {
    start_transaction: AsyncJsFunction<(), JsTransaction>,
}

/// This a JS proxy for accessing the methods, specific
/// to JS transaction objects
#[wasm_bindgen(getter_with_clone)]
pub(crate) struct TransactionProxy {
    /// transaction options
    options: TransactionOptions,

    /// commit transaction
    commit: AsyncJsFunction<(), ()>,

    /// rollback transaction
    rollback: AsyncJsFunction<(), ()>,

    /// whether the transaction has already been committed or rolled back
    closed: AtomicBool,
}

impl CommonProxy {
    pub fn new(object: &JsObjectExtern) -> JsResult<Self> {
        let flavour: String = JsString::from(object.get("flavour".into())?).into();

        Ok(Self {
            query_raw: JsFunction::from(object.get("queryRaw".into())?).into(),
            execute_raw: JsFunction::from(object.get("executeRaw".into())?).into(),
            flavour,
        })
    }

    pub async fn query_raw(&self, params: Query) -> quaint::Result<JSResultSet> {
        self.query_raw.call(params).await
    }

    pub async fn execute_raw(&self, params: Query) -> quaint::Result<u32> {
        self.execute_raw.call(params).await
    }
}

impl DriverProxy {
    pub fn new(object: &JsObjectExtern) -> JsResult<Self> {
        Ok(Self {
            start_transaction: JsFunction::from(object.get("startTransaction".into())?).into(),
        })
    }

    async fn start_transaction_inner(&self) -> quaint::Result<Box<JsTransaction>> {
        let tx = self.start_transaction.call(()).await?;

        // Decrement for this gauge is done in JsTransaction::commit/JsTransaction::rollback
        // Previously, it was done in JsTransaction::new, similar to the native Transaction.
        // However, correct Dispatcher is lost there and increment does not register, so we moved
        // it here instead.
        increment_gauge!("prisma_client_queries_active", 1.0);
        Ok(Box::new(tx))
    }

    pub fn start_transaction<'a>(
        &'a self,
    ) -> SendFuture<impl Future<Output = quaint::Result<Box<JsTransaction>>> + 'a> {
        SendFuture(self.start_transaction_inner())
    }
}

impl TransactionProxy {
    pub fn new(object: &JsObjectExtern) -> JsResult<Self> {
        let options = object.get("options".into())?;
        let closed = AtomicBool::new(false);

        Ok(Self {
            options: TransactionOptions::from_js(options).unwrap(),
            commit: JsFunction::from(object.get("commit".into())?).into(),
            rollback: JsFunction::from(object.get("rollback".into())?).into(),
            closed,
        })
    }

    pub fn options(&self) -> &TransactionOptions {
        &self.options
    }

    pub fn commit<'a>(&'a self) -> SendFuture<impl Future<Output = quaint::Result<()>> + 'a> {
        self.closed.store(true, Ordering::Relaxed);
        SendFuture(self.commit.call(()))
    }

    pub fn rollback<'a>(&'a self) -> SendFuture<impl Future<Output = quaint::Result<()>> + 'a> {
        self.closed.store(true, Ordering::Relaxed);
        SendFuture(self.rollback.call(()))
    }
}

impl Drop for TransactionProxy {
    fn drop(&mut self) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }

        _ = self.rollback.call_non_blocking(());
    }
}

// Assume the proxy object will not be sent to service workers, we can unsafe impl Send + Sync.
unsafe impl Send for TransactionProxy {}
unsafe impl Sync for TransactionProxy {}

unsafe impl Send for DriverProxy {}
unsafe impl Sync for DriverProxy {}

unsafe impl Send for CommonProxy {}
unsafe impl Sync for CommonProxy {}
