use std::{
    hash::{BuildHasher, Hash, RandomState},
    sync::Arc,
};

use async_trait::async_trait;
use futures::lock::Mutex;
use lru_cache::LruCache;
use postgres_types::Type;
use tokio_postgres::{Client, Error, Statement};

use crate::connector::metrics::strip_query_traceparent;

use super::query::{PreparedQuery, TypedQuery};

/// Types that can be used as a cache for prepared queries.
#[async_trait]
pub trait QueryCache: From<CacheSettings> + Send + Sync {
    /// The type of query that is returned by the cache.
    type Query: PreparedQuery;

    /// Retrieve a prepared query from the cache or prepare and cache one if it's not present.
    async fn get_by_query(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Self::Query, Error>;
}

/// A no-op cache that creates a new prepared statement for each query.
/// Useful when we don't need caching.
#[derive(Debug, Default)]
pub struct NoopPreparedStatementCache;

#[async_trait]
impl QueryCache for NoopPreparedStatementCache {
    type Query = Statement;

    #[inline]
    async fn get_by_query(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Statement, Error> {
        client.prepare_typed(sql, types).await
    }
}

impl From<CacheSettings> for NoopPreparedStatementCache {
    fn from(_: CacheSettings) -> Self {
        Self
    }
}

/// An LRU cache that creates and stores prepared statements.
#[derive(Debug)]
pub struct LruPreparedStatementCache {
    cache: InnerLruCache<Statement>,
}

impl LruPreparedStatementCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: InnerLruCache::with_capacity(capacity),
        }
    }
}

#[async_trait]
impl QueryCache for LruPreparedStatementCache {
    type Query = Statement;

    async fn get_by_query(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Statement, Error> {
        match self.cache.get(sql, types).await {
            Some(statement) => Ok(statement),
            None => {
                let stmt = client.prepare_typed(sql, types).await?;
                self.cache.insert(sql, types, stmt.clone()).await;
                Ok(stmt)
            }
        }
    }
}

impl From<CacheSettings> for LruPreparedStatementCache {
    fn from(settings: CacheSettings) -> Self {
        Self::with_capacity(settings.capacity)
    }
}

/// An LRU cache that creates and stores type information relevant to each query, with keys being
/// stripped of any tracing information.
///
/// Returns [`TypedQuery`] instances, rather than [`Statement`], because prepared statements cannot
/// be re-used when the tracing information is attached to them.
#[derive(Debug)]
pub struct LruTracingCache {
    cache: InnerLruCache<Arc<TypedQuery>>,
}

impl LruTracingCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: InnerLruCache::with_capacity(capacity),
        }
    }
}

#[async_trait]
impl QueryCache for LruTracingCache {
    type Query = Arc<TypedQuery>;

    async fn get_by_query(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Arc<TypedQuery>, Error> {
        let sql_without_traceparent = strip_query_traceparent(sql);

        match self.cache.get(sql_without_traceparent, types).await {
            Some(query) => Ok(query),
            None => {
                let stmt = client.prepare_typed(sql, types).await?;
                let query = Arc::new(TypedQuery::from_statement(sql, &stmt));
                self.cache.insert(sql_without_traceparent, types, query.clone()).await;
                Ok(query)
            }
        }
    }
}

impl From<CacheSettings> for LruTracingCache {
    fn from(settings: CacheSettings) -> Self {
        Self::with_capacity(settings.capacity)
    }
}

/// Settings related to query caching.
#[derive(Debug)]
pub struct CacheSettings {
    pub capacity: usize,
}

/// Key uniquely representing an SQL statement in the prepared statements cache.
#[derive(Debug, PartialEq, Eq, Hash)]
struct QueryKey {
    /// Hash of a string with SQL query.
    sql: u64,
    /// Combined hash of types for all parameters from the query.
    types_hash: u64,
}

impl QueryKey {
    fn new<S: BuildHasher>(st: &S, sql: &str, params: &[Type]) -> Self {
        Self {
            sql: st.hash_one(sql),
            types_hash: st.hash_one(params),
        }
    }
}

#[derive(Debug)]
struct InnerLruCache<V> {
    cache: Mutex<LruCache<QueryKey, V>>,
    state: RandomState,
}

impl<V> InnerLruCache<V> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
            state: RandomState::new(),
        }
    }

    async fn get(&self, sql: &str, types: &[Type]) -> Option<V>
    where
        V: Clone,
    {
        let mut cache = self.cache.lock().await;
        let capacity = cache.capacity();
        let stored = cache.len();

        let key = QueryKey::new(&self.state, sql, types);
        match cache.get_mut(&key) {
            Some(value) => {
                tracing::trace!(
                    message = "CACHE HIT!",
                    query = sql,
                    capacity = capacity,
                    stored = stored,
                );
                Some(value.clone())
            }
            None => {
                tracing::trace!(
                    message = "CACHE MISS!",
                    query = sql,
                    capacity = capacity,
                    stored = stored,
                );
                None
            }
        }
    }

    pub async fn insert(&self, sql: &str, types: &[Type], value: V) {
        let key = QueryKey::new(&self.state, sql, types);
        self.cache.lock().await.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;

    pub(crate) use crate::connector::postgres::url::PostgresNativeUrl;
    use crate::{
        connector::{MakeTlsConnectorManager, PostgresFlavour},
        tests::test_api::postgres::CONN_STR,
    };
    use url::Url;

    #[tokio::test]
    async fn noop_prepared_statement_cache_prepares_new_statements_every_time() {
        run_with_client(|client| async move {
            let cache = NoopPreparedStatementCache;
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let stmt1 = cache.get_by_query(&client, sql, &types).await.unwrap();
            let stmt2 = cache.get_by_query(&client, sql, &types).await.unwrap();
            assert_ne!(stmt1.name(), stmt2.name());
        })
        .await;
    }

    #[tokio::test]
    async fn lru_prepared_statement_cache_reuses_statements_within_capacity() {
        run_with_client(|client| async move {
            let cache = LruPreparedStatementCache::with_capacity(1);
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let stmt1 = cache.get_by_query(&client, sql, &types).await.unwrap();
            let stmt2 = cache.get_by_query(&client, sql, &types).await.unwrap();
            assert_eq!(stmt1.name(), stmt2.name());

            // replace our cached statement with a new one going over the capacity
            cache.get_by_query(&client, sql, &[Type::INT8]).await.unwrap();

            // the old statement should be evicted from the cache
            let stmt3 = cache.get_by_query(&client, sql, &types).await.unwrap();
            assert_ne!(stmt1.name(), stmt3.name());
        })
        .await;
    }

    #[tokio::test]
    async fn tracing_cache_reuses_queries_within_capacity() {
        run_with_client(|client| async move {
            let cache = LruTracingCache::with_capacity(1);
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let stmt1 = cache.get_by_query(&client, sql, &types).await.unwrap();
            let stmt2 = cache.get_by_query(&client, sql, &types).await.unwrap();
            assert!(Arc::ptr_eq(&stmt1, &stmt2), "stmt1 and stmt2 should be the same Arc");

            // replace our cached query with a new one going over the capacity
            cache.get_by_query(&client, sql, &[Type::INT8]).await.unwrap();

            // the old query should be evicted from the cache
            let stmt3 = cache.get_by_query(&client, sql, &types).await.unwrap();
            assert!(
                !Arc::ptr_eq(&stmt1, &stmt3),
                "stmt1 and stmt3 should not be the same Arc"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn tracing_cache_reuses_queries_with_different_traceparent() {
        run_with_client(|client| async move {
            let cache = LruTracingCache::with_capacity(1);
            let sql1 = "SELECT $1 /* traceparent=00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01 */";
            let sql2 = "SELECT $1 /* traceparent=00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-02 */";
            let types = [Type::INT4];

            let stmt1 = cache.get_by_query(&client, sql1, &types).await.unwrap();
            let stmt2 = cache.get_by_query(&client, sql2, &types).await.unwrap();
            assert!(Arc::ptr_eq(&stmt1, &stmt2), "stmt1 and stmt2 should be the same Arc");
        })
        .await;
    }

    async fn run_with_client<Func, Fut>(test: Func)
    where
        Func: FnOnce(Client) -> Fut,
        Fut: Future<Output = ()>,
    {
        let url = Url::parse(&CONN_STR).unwrap();
        let mut pg_url = PostgresNativeUrl::new(url).unwrap();
        pg_url.set_flavour(PostgresFlavour::Postgres);

        let tls_manager = MakeTlsConnectorManager::new(pg_url.clone());
        let tls = tls_manager.get_connector().await.unwrap();

        let (client, conn) = pg_url.to_config().connect(tls).await.unwrap();

        let set = tokio::task::LocalSet::new();
        set.spawn_local(conn);
        set.run_until(test(client)).await
    }
}
