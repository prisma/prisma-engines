use crate::{
    conversion,
    proxy::{CommonProxy, DriverProxy, Query},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use napi::JsObject;
use psl::datamodel_connector::Flavour;
use quaint::{
    connector::{metrics, IsolationLevel, Transaction},
    error::{Error, ErrorKind},
    prelude::{Query as QuaintQuery, Queryable as QuaintQueryable, ResultSet, TransactionCapable},
    visitor::{self, Visitor},
};
use std::sync::Arc;
use tracing::{info_span, Instrument};

/// A JsQueryable adapts a Proxy to implement quaint's Queryable interface. It has the
/// responsibility of transforming inputs and outputs of `query` and `execute` methods from quaint
/// types to types that can be translated into javascript and viceversa. This is to let the rest of
/// the query engine work as if it was using quaint itself. The aforementioned transformations are:
///
/// Transforming a `quaint::ast::Query` into SQL by visiting it for the specific flavour of SQL
/// expected by the client connector. (eg. using the mysql visitor for the Planetscale client
/// connector)
///
/// Transforming a `JSResultSet` (what client connectors implemented in javascript provide)
/// into a `quaint::connector::result_set::ResultSet`. A quaint `ResultSet` is basically a vector
/// of `quaint::Value` but said type is a tagged enum, with non-unit variants that cannot be converted to javascript as is.
///
pub(crate) struct JsBaseQueryable {
    pub(crate) proxy: CommonProxy,
    pub flavour: Flavour,
}

impl JsBaseQueryable {
    pub(crate) fn new(proxy: CommonProxy) -> Self {
        let flavour: Flavour = proxy.flavour.parse().unwrap();
        Self { proxy, flavour }
    }

    /// visit a quaint query AST according to the flavour of the JS connector
    fn visit_quaint_query<'a>(&self, q: QuaintQuery<'a>) -> quaint::Result<(String, Vec<quaint::Value<'a>>)> {
        match self.flavour {
            Flavour::Mysql => visitor::Mysql::build(q),
            Flavour::Postgres => visitor::Postgres::build(q),
            Flavour::Sqlite => visitor::Sqlite::build(q),
            _ => unimplemented!("Unsupported flavour for JS connector {:?}", self.flavour),
        }
    }

    async fn build_query(&self, sql: &str, values: &[quaint::Value<'_>]) -> quaint::Result<Query> {
        let sql: String = sql.to_string();

        let converter = match self.flavour {
            Flavour::Postgres => conversion::postgres::value_to_js_arg,
            Flavour::Sqlite => conversion::sqlite::value_to_js_arg,
            Flavour::Mysql => conversion::mysql::value_to_js_arg,
            _ => unreachable!("Unsupported flavour for JS connector {:?}", self.flavour),
        };

        let args = values
            .iter()
            .map(converter)
            .collect::<serde_json::Result<Vec<conversion::JSArg>>>()?;

        Ok(Query { sql, args })
    }
}

#[async_trait]
impl QuaintQueryable for JsBaseQueryable {
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        let (sql, params) = self.visit_quaint_query(q)?;
        self.query_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        metrics::query("js.query_raw", sql, params, move || async move {
            self.do_query_raw(sql, params).await
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        let (sql, params) = self.visit_quaint_query(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<u64> {
        metrics::query("js.execute_raw", sql, params, move || async move {
            self.do_execute_raw(sql, params).await
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<u64> {
        self.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        let params = &[];
        metrics::query("js.raw_cmd", cmd, params, move || async move {
            self.do_execute_raw(cmd, params).await?;
            Ok(())
        })
        .await
    }

    async fn version(&self) -> quaint::Result<Option<String>> {
        // Note: JS Connectors don't use this method.
        Ok(None)
    }

    fn is_healthy(&self) -> bool {
        // Note: JS Connectors don't use this method.
        true
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        if matches!(isolation_level, IsolationLevel::Snapshot) {
            return Err(Error::builder(ErrorKind::invalid_isolation_level(&isolation_level)).build());
        }

        if self.flavour == Flavour::Sqlite {
            return match isolation_level {
                IsolationLevel::Serializable => Ok(()),
                _ => Err(Error::builder(ErrorKind::invalid_isolation_level(&isolation_level)).build()),
            };
        }

        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await
    }

    fn requires_isolation_first(&self) -> bool {
        match self.flavour {
            Flavour::Mysql => true,
            Flavour::Postgres | Flavour::Sqlite => false,
            _ => unreachable!(),
        }
    }
}

impl JsBaseQueryable {
    pub fn phantom_query_message(stmt: &str) -> String {
        format!(r#"-- Implicit "{}" query via underlying driver"#, stmt)
    }

    async fn do_query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = self.build_query(sql, params).instrument(serialization_span).await?;

        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let result_set = self.proxy.query_raw(query).instrument(sql_span).await?;

        let len = result_set.len();
        let _deserialization_span = info_span!("js:query:result", user_facing = true, "length" = %len).entered();

        result_set.try_into()
    }

    async fn do_execute_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<u64> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = self.build_query(sql, params).instrument(serialization_span).await?;

        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let affected_rows = self.proxy.execute_raw(query).instrument(sql_span).await?;

        Ok(affected_rows as u64)
    }
}

/// A JsQueryable adapts a Proxy to implement quaint's Queryable interface. It has the
/// responsibility of transforming inputs and outputs of `query` and `execute` methods from quaint
/// types to types that can be translated into javascript and viceversa. This is to let the rest of
/// the query engine work as if it was using quaint itself. The aforementioned transformations are:
///
/// Transforming a `quaint::ast::Query` into SQL by visiting it for the specific flavour of SQL
/// expected by the client connector. (eg. using the mysql visitor for the Planetscale client
/// connector)
///
/// Transforming a `JSResultSet` (what client connectors implemented in javascript provide)
/// into a `quaint::connector::result_set::ResultSet`. A quaint `ResultSet` is basically a vector
/// of `quaint::Value` but said type is a tagged enum, with non-unit variants that cannot be converted to javascript as is.
///
pub struct JsQueryable {
    inner: JsBaseQueryable,
    driver_proxy: DriverProxy,
    pub transaction_depth: Arc<Mutex<i32>>,
}

impl std::fmt::Display for JsQueryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSQueryable(driver)")
    }
}

impl std::fmt::Debug for JsQueryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSQueryable(driver)")
    }
}

#[async_trait]
impl QuaintQueryable for JsQueryable {
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        self.inner.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw_typed(sql, params).await
    }

    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        self.inner.execute(q).await
    }

    async fn execute_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<u64> {
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

#[async_trait]
impl TransactionCapable for JsQueryable {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> quaint::Result<Box<dyn Transaction + 'a>> {
        let tx = self.driver_proxy.start_transaction().await?;

        let isolation_first = tx.requires_isolation_first();

        if isolation_first {
            if let Some(isolation) = isolation {
                tx.set_tx_isolation_level(isolation).await?;
            }
        }

        let mut depth_guard = self.transaction_depth.lock().await;
        *depth_guard += 1;

        let st_depth = *depth_guard;

        let begin_stmt = tx.begin_statement(st_depth).await;

        let tx_opts = tx.options();
        if tx_opts.use_phantom_query {
            let begin_stmt = JsBaseQueryable::phantom_query_message(&begin_stmt);
            tx.raw_phantom_cmd(begin_stmt.as_str()).await?;
        } else {
            tx.raw_cmd(&begin_stmt).await?;
        }

        if !isolation_first {
            if let Some(isolation) = isolation {
                tx.set_tx_isolation_level(isolation).await?;
            }
        }

        self.server_reset_query(tx.as_ref()).await?;

        Ok(tx)
    }
}

pub fn from_napi(driver: JsObject) -> JsQueryable {
    let common = CommonProxy::new(&driver).unwrap();
    let driver_proxy = DriverProxy::new(&driver).unwrap();

    JsQueryable {
        inner: JsBaseQueryable::new(common),
        driver_proxy,
        transaction_depth: Arc::new(futures::lock::Mutex::new(0)),
    }
}
