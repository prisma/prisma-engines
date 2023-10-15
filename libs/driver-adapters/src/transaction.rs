use std::borrow::Cow;

use async_trait::async_trait;
use quaint::{
    Value,
    connector::{DescribedQuery, IsolationLevel, Transaction as QuaintTransaction},
    prelude::{Query as QuaintQuery, Queryable, ResultSet},
};

use crate::proxy::{TransactionOptions, TransactionProxy};
use crate::{JsObject, JsResult};
use crate::{proxy::CommonProxy, queryable::JsBaseQueryable, send_future::UnsafeFuture};

// Wrapper around JS transaction objects that implements Queryable
// and quaint::Transaction. Can be used in place of quaint transaction,
// but delegates most operations to JS
pub(crate) struct JsTransaction {
    tx_proxy: TransactionProxy,
    inner: JsBaseQueryable,
    depth: u32,
}

impl JsTransaction {
    pub(crate) fn new(inner: JsBaseQueryable, tx_proxy: TransactionProxy) -> Self {
        Self {
            inner,
            tx_proxy,
            depth: 0,
        }
    }

    pub fn options(&self) -> &TransactionOptions {
        self.tx_proxy.options()
    }

    pub async fn raw_phantom_cmd(&self, cmd: &str) -> quaint::Result<()> {
        let params = &[];
        quaint::connector::trace::query(self.inner.db_system_name, cmd, params, move || async move { Ok(()) }).await
    }
}

#[async_trait]
impl QuaintTransaction for JsTransaction {
    async fn begin(&mut self) -> quaint::Result<()> {
        self.depth += 1;
        let begin_stmt = self.begin_statement(self.depth);

        let begin_res = if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(begin_stmt.as_ref());
            self.raw_phantom_cmd(phantom.as_str()).await
        } else {
            self.inner.raw_cmd(begin_stmt.as_ref()).await
        };

        if let Err(err) = begin_res {
            // Keep depth consistent with the underlying driver if we failed to begin.
            self.depth -= 1;
            return Err(err);
        }

        UnsafeFuture(self.tx_proxy.begin()).await?;
        Ok(())
    }

    async fn commit(&mut self) -> quaint::Result<u32> {
        let commit_stmt = self.commit_statement(self.depth);

        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(commit_stmt.as_ref());
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(commit_stmt.as_ref()).await?;
        }

        UnsafeFuture(self.tx_proxy.commit()).await?;

        // Modify the depth value
        self.depth -= 1;

        Ok(self.depth)
    }

    async fn rollback(&mut self) -> quaint::Result<u32> {
        let rollback_stmt = self.rollback_statement(self.depth);

        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(rollback_stmt.as_ref());
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(rollback_stmt.as_ref()).await?;
        }

        UnsafeFuture(self.tx_proxy.rollback()).await?;

        // Modify the depth value
        self.depth -= 1;

        Ok(self.depth)
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

    async fn describe_query(&self, sql: &str) -> quaint::Result<DescribedQuery> {
        self.inner.describe_query(sql).await
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

    fn begin_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.begin_statement(depth)
    }

    fn commit_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.commit_statement(depth)
    }

    fn rollback_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.rollback_statement(depth)
    }
}

impl super::wasm::FromJsValue for JsTransaction {
    fn from_js_value(value: wasm_bindgen::prelude::JsValue) -> JsResult<Self> {
        use wasm_bindgen::JsCast;

        let object = value.dyn_into::<JsObject>()?;
        let common_proxy = CommonProxy::new(&object)?;
        let base = JsBaseQueryable::new(common_proxy);
        let tx_proxy = TransactionProxy::new(&object)?;

        Ok(Self::new(base, tx_proxy))
    }
}
