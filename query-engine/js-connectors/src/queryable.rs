use crate::{
    error::into_quaint_error,
    proxy::{CommonProxy, DriverProxy, Query},
};
use async_trait::async_trait;
use napi::{Env, JsObject};
use psl::datamodel_connector::Flavour;
use quaint::{
    connector::{IsolationLevel, Transaction},
    error::{Error, ErrorKind},
    prelude::{Query as QuaintQuery, Queryable as QuaintQueryable, ResultSet, TransactionCapable},
    visitor::{self, Visitor},
    Value,
};
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
pub struct JsBaseQueryable {
    pub(crate) proxy: CommonProxy,
    pub flavour: Flavour,
}

impl JsBaseQueryable {
    pub fn new(proxy: CommonProxy) -> Self {
        let flavour: Flavour = proxy.flavour.to_owned().parse().unwrap();
        Self { proxy, flavour }
    }

    /// visit a query according to the flavour of the JS connector
    pub fn visit_query<'a>(&self, q: QuaintQuery<'a>) -> quaint::Result<(String, Vec<Value<'a>>)> {
        match self.flavour {
            Flavour::Mysql => visitor::Mysql::build(q),
            Flavour::Postgres => visitor::Postgres::build(q),
            _ => unimplemented!("Unsupported flavour for JS connector {:?}", self.flavour),
        }
    }
}

#[async_trait]
impl QuaintQueryable for JsBaseQueryable {
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        let (sql, params) = self.visit_query(q)?;
        self.query_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        let span = info_span!("js:query", user_facing = true);
        self.do_query_raw(sql, params).instrument(span).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        let (sql, params) = self.visit_query(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        let span = info_span!("js:query", user_facing = true);
        self.do_execute_raw(sql, params).instrument(span).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        self.execute_raw(cmd, &[]).await?;
        Ok(())
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
    async fn build_query(sql: &str, values: &[quaint::Value<'_>]) -> Query {
        let sql: String = sql.to_string();
        let args = values.iter().map(|v| v.clone().into()).collect();
        Query { sql, args }
    }

    async fn do_query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = Self::build_query(sql, params).instrument(serialization_span).await;

        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let result_set = self
            .proxy
            .query_raw(query)
            .instrument(sql_span)
            .await
            .map_err(into_quaint_error)?;

        let len = result_set.len();
        let _deserialization_span = info_span!("js:query:result", user_facing = true, "length" = %len).entered();
        Ok(ResultSet::from(result_set))
    }

    async fn do_execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = Self::build_query(sql, params).instrument(serialization_span).await;

        // Todo: convert napi::Error to quaint::error::Error.
        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let affected_rows = self.proxy.execute_raw(query).instrument(sql_span).await.unwrap();

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

#[async_trait]
impl TransactionCapable for JsQueryable {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> quaint::Result<Box<dyn Transaction + 'a>> {
        let tx = self
            .driver_proxy
            .start_transaction(isolation)
            .await
            .map_err(into_quaint_error)?;

        Ok(tx)
    }
}

pub fn from_napi(napi_env: &Env, driver: JsObject) -> JsQueryable {
    let common = CommonProxy::new(&driver, napi_env).unwrap();
    let driver_proxy = DriverProxy::new(&driver, napi_env).unwrap();

    JsQueryable {
        inner: JsBaseQueryable::new(common),
        driver_proxy,
    }
}
