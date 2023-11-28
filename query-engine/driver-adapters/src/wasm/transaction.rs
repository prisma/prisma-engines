use async_trait::async_trait;
use js_sys::Object as JsObject;
use metrics::decrement_gauge;
use quaint::{
    connector::{IsolationLevel, Transaction as QuaintTransaction},
    prelude::{Query as QuaintQuery, Queryable, ResultSet},
    Value,
};
use wasm_bindgen::JsCast;

use super::from_js::FromJsValue;
use crate::proxy::{TransactionOptions, TransactionProxy};
use crate::{proxy::CommonProxy, queryable::JsBaseQueryable, send_future::SendFuture, JsObjectExtern};

// Wrapper around JS transaction objects that implements Queryable
// and quaint::Transaction. Can be used in place of quaint transaction,
// but delegates most operations to JS
pub(crate) struct JsTransaction {
    tx_proxy: TransactionProxy,
    inner: JsBaseQueryable,
}

impl JsTransaction {
    pub(crate) fn new(inner: JsBaseQueryable, tx_proxy: TransactionProxy) -> Self {
        Self { inner, tx_proxy }
    }

    pub fn options(&self) -> &TransactionOptions {
        self.tx_proxy.options()
    }

    pub async fn raw_phantom_cmd(&self, cmd: &str) -> quaint::Result<()> {
        let params = &[];
        quaint::connector::metrics::query("js.raw_phantom_cmd", cmd, params, move || async move { Ok(()) }).await
    }
}

impl FromJsValue for JsTransaction {
    fn from_js_value(value: wasm_bindgen::prelude::JsValue) -> Result<Self, wasm_bindgen::prelude::JsValue> {
        let object: JsObjectExtern = value.dyn_into::<JsObject>()?.unchecked_into();
        let common_proxy = CommonProxy::new(&object)?;
        let base = JsBaseQueryable::new(common_proxy);
        let tx_proxy = TransactionProxy::new(&object)?;

        Ok(Self::new(base, tx_proxy))
    }
}

#[async_trait]
impl QuaintTransaction for JsTransaction {
    async fn commit(&self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let commit_stmt = "COMMIT";

        if self.options().use_phantom_query {
            let commit_stmt = JsBaseQueryable::phantom_query_message(commit_stmt);
            self.raw_phantom_cmd(commit_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(commit_stmt).await?;
        }

        SendFuture(self.tx_proxy.commit()).await
    }

    async fn rollback(&self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let rollback_stmt = "ROLLBACK";

        if self.options().use_phantom_query {
            let rollback_stmt = JsBaseQueryable::phantom_query_message(rollback_stmt);
            self.raw_phantom_cmd(rollback_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(rollback_stmt).await?;
        }

        SendFuture(self.tx_proxy.rollback()).await
    }

    fn as_queryable(&self) -> &dyn Queryable {
        self
    }
}

#[async_trait]
impl Queryable for JsTransaction {
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        self.inner.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw_typed(sql, params).await
    }

    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        self.inner.execute(q).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw_typed(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        self.inner.raw_cmd(cmd).await
    }

    async fn version(&self) -> quaint::Result<Option<String>> {
        self.inner.version().await
    }

    fn is_healthy(&self) -> bool {
        self.inner.is_healthy()
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        self.inner.set_tx_isolation_level(isolation_level).await
    }

    fn requires_isolation_first(&self) -> bool {
        self.inner.requires_isolation_first()
    }
}

// Assume the proxy object will not be sent to service workers, we can unsafe impl Send + Sync.
unsafe impl Send for JsTransaction {}
unsafe impl Sync for JsTransaction {}
