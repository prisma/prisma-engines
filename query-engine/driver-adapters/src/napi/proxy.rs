pub use crate::types::{ColumnType, JSResultSet, Query, TransactionOptions};

use super::async_js_function::AsyncJsFunction;
use super::transaction::JsTransaction;
use metrics::increment_gauge;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi::{JsObject, JsString};

/// Proxy is a struct wrapping a javascript object that exhibits basic primitives for
/// querying and executing SQL (i.e. a client connector). The Proxy uses NAPI ThreadSafeFunction to
/// invoke the code within the node runtime that implements the client connector.
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
pub(crate) struct DriverProxy {
    start_transaction: AsyncJsFunction<(), JsTransaction>,
}
/// This a JS proxy for accessing the methods, specific
/// to JS transaction objects
pub(crate) struct TransactionProxy {
    /// transaction options
    options: TransactionOptions,

    /// commit transaction
    commit: AsyncJsFunction<(), ()>,

    /// rollback transaction
    rollback: AsyncJsFunction<(), ()>,

    /// dispose transaction, cleanup logic executed at the end of the transaction lifecycle
    /// on drop.
    dispose: ThreadsafeFunction<(), ErrorStrategy::Fatal>,
}

impl CommonProxy {
    pub fn new(object: &JsObject) -> napi::Result<Self> {
        let flavour: JsString = object.get_named_property("flavour")?;

        Ok(Self {
            query_raw: object.get_named_property("queryRaw")?,
            execute_raw: object.get_named_property("executeRaw")?,
            flavour: flavour.into_utf8()?.as_str()?.to_owned(),
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
    pub fn new(driver_adapter: &JsObject) -> napi::Result<Self> {
        Ok(Self {
            start_transaction: driver_adapter.get_named_property("startTransaction")?,
        })
    }

    pub async fn start_transaction(&self) -> quaint::Result<Box<JsTransaction>> {
        let tx = self.start_transaction.call(()).await?;

        // Decrement for this gauge is done in JsTransaction::commit/JsTransaction::rollback
        // Previously, it was done in JsTransaction::new, similar to the native Transaction.
        // However, correct Dispatcher is lost there and increment does not register, so we moved
        // it here instead.
        increment_gauge!("prisma_client_queries_active", 1.0);
        Ok(Box::new(tx))
    }
}

impl TransactionProxy {
    pub fn new(js_transaction: &JsObject) -> napi::Result<Self> {
        let commit = js_transaction.get_named_property("commit")?;
        let rollback = js_transaction.get_named_property("rollback")?;
        let dispose = js_transaction.get_named_property("dispose")?;
        let options = js_transaction.get_named_property("options")?;

        Ok(Self {
            commit,
            rollback,
            dispose,
            options,
        })
    }

    pub fn options(&self) -> &TransactionOptions {
        &self.options
    }

    pub async fn commit(&self) -> quaint::Result<()> {
        self.commit.call(()).await
    }

    pub async fn rollback(&self) -> quaint::Result<()> {
        self.rollback.call(()).await
    }
}

impl Drop for TransactionProxy {
    fn drop(&mut self) {
        _ = self
            .dispose
            .call((), napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking);
    }
}
