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

use super::query::{PreparedQuery, QueryMetadata, TypedQuery};

/// Types that can be used as a cache for prepared queries and statements.
#[async_trait]
pub trait QueryCache: From<CacheSettings> + Send + Sync {
    /// The type that is returned when a prepared query is requested from the cache.
    type Query<'a>: PreparedQuery;

    /// Retrieve a prepared query.
    async fn get_query<'a>(&self, client: &Client, sql: &'a str, types: &[Type]) -> Result<Self::Query<'a>, Error>;

    /// Retrieve a prepared statement.
    ///
    /// This is useful in scenarios that require direct access to a prepared statement,
    /// e.g. describing a query.
    async fn get_statement(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Statement, Error>;
}

/// A no-op cache that creates a new prepared statement for every requested query.
/// Useful when we don't need caching.
#[derive(Debug, Default)]
pub struct NoOpCache;

#[async_trait]
impl QueryCache for NoOpCache {
    type Query<'a> = Statement;

    #[inline]
    async fn get_query<'a>(&self, client: &Client, sql: &'a str, types: &[Type]) -> Result<Statement, Error> {
        self.get_statement(client, sql, types).await
    }

    #[inline]
    async fn get_statement(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Statement, Error> {
        client.prepare_typed(sql, types).await
    }
}

impl From<CacheSettings> for NoOpCache {
    fn from(_: CacheSettings) -> Self {
        Self
    }
}

/// An LRU cache that creates a prepared statement for every query that is not in the cache.
#[derive(Debug)]
pub struct PreparedStatementLruCache {
    cache: InnerLruCache<Statement>,
}

impl PreparedStatementLruCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: InnerLruCache::with_capacity(capacity),
        }
    }
}

#[async_trait]
impl QueryCache for PreparedStatementLruCache {
    type Query<'a> = Statement;

    #[inline]
    async fn get_query<'a>(&self, client: &Client, sql: &'a str, types: &[Type]) -> Result<Statement, Error> {
        self.get_statement(client, sql, types).await
    }

    async fn get_statement(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Statement, Error> {
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

impl From<CacheSettings> for PreparedStatementLruCache {
    fn from(settings: CacheSettings) -> Self {
        Self::with_capacity(settings.capacity)
    }
}

/// An LRU cache that creates and stores query type information rather than prepared statements.
/// Queries are identified by their content with tracing information removed (which makes it
/// possible to cache traced queries at all) and returned as instances of [`TypedQuery`]. The
/// caching behavior is implemented in [`get_query`](Self::get_query), while statements returned
/// from [`get_statement`](Self::get_statement) are always freshly prepared, because statements
/// cannot be re-used when tracing information is present.
#[derive(Debug)]
pub struct TracingLruCache {
    cache: InnerLruCache<Arc<QueryMetadata>>,
}

impl TracingLruCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: InnerLruCache::with_capacity(capacity),
        }
    }
}

#[async_trait]
impl QueryCache for TracingLruCache {
    type Query<'a> = TypedQuery<'a>;

    async fn get_query<'a>(&self, client: &Client, sql: &'a str, types: &[Type]) -> Result<TypedQuery<'a>, Error> {
        let sql_without_traceparent = strip_query_traceparent(sql);

        let metadata = match self.cache.get(sql_without_traceparent, types).await {
            Some(metadata) => metadata,
            None => {
                let stmt = client.prepare_typed(sql_without_traceparent, types).await?;
                let metdata = Arc::new(QueryMetadata::from(&stmt));
                self.cache.insert(sql_without_traceparent, types, metdata.clone()).await;
                metdata
            }
        };
        Ok(TypedQuery::from_sql_and_metadata(sql, metadata))
    }

    async fn get_statement(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Statement, Error> {
        client.prepare_typed(sql, types).await
    }
}

impl From<CacheSettings> for TracingLruCache {
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
        // we call `get_mut` because LRU requires mutable access for lookups
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
    async fn noop_cache_returns_new_queries_every_time() {
        run_with_client(|client| async move {
            let cache = NoOpCache;
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let stmt1 = cache.get_query(&client, sql, &types).await.unwrap();
            let stmt2 = cache.get_query(&client, sql, &types).await.unwrap();
            assert_ne!(stmt1.name(), stmt2.name());
        })
        .await;
    }

    #[tokio::test]
    async fn noop_cache_returns_new_statements_every_time() {
        run_with_client(|client| async move {
            let cache = NoOpCache;
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let stmt1 = cache.get_statement(&client, sql, &types).await.unwrap();
            let stmt2 = cache.get_statement(&client, sql, &types).await.unwrap();
            assert_ne!(stmt1.name(), stmt2.name());
        })
        .await;
    }

    #[tokio::test]
    async fn prepared_statement_lru_cache_reuses_queries_within_capacity() {
        run_with_client(|client| async move {
            let cache = PreparedStatementLruCache::with_capacity(3);
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let stmt1 = cache.get_query(&client, sql, &types).await.unwrap();
            let stmt2 = cache.get_query(&client, sql, &types).await.unwrap();
            assert_eq!(stmt1.name(), stmt2.name());

            // fill the cache with different types, causing the first query to be evicted
            for typ in [Type::INT8, Type::INT4_ARRAY, Type::INT8_ARRAY] {
                cache.get_query(&client, sql, &[typ]).await.unwrap();
            }

            // the old statement should be re-created
            let stmt3 = cache.get_query(&client, sql, &types).await.unwrap();
            assert_ne!(stmt1.name(), stmt3.name());
        })
        .await;
    }

    #[tokio::test]
    async fn prepared_statement_lru_cache_reuses_statements_within_capacity() {
        run_with_client(|client| async move {
            let cache = PreparedStatementLruCache::with_capacity(3);
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let stmt1 = cache.get_statement(&client, sql, &types).await.unwrap();
            let stmt2 = cache.get_statement(&client, sql, &types).await.unwrap();
            assert_eq!(stmt1.name(), stmt2.name());

            // fill the cache with different types, causing the first query to be evicted
            for typ in [Type::INT8, Type::INT4_ARRAY, Type::INT8_ARRAY] {
                cache.get_query(&client, sql, &[typ]).await.unwrap();
            }

            // the old statement should be re-created
            let stmt3 = cache.get_statement(&client, sql, &types).await.unwrap();
            assert_ne!(stmt1.name(), stmt3.name());
        })
        .await;
    }

    #[tokio::test]
    async fn tracing_lru_cache_reuses_queries_within_capacity() {
        run_with_client(|client| async move {
            let cache = TracingLruCache::with_capacity(3);
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let q1 = cache.get_query(&client, sql, &types).await.unwrap();
            let q2 = cache.get_query(&client, sql, &types).await.unwrap();
            assert!(
                Arc::ptr_eq(&q1.metadata, &q2.metadata),
                "q1 and q2 should re-use the same metadata"
            );

            // fill the cache with different types, causing the first query to be evicted
            for typ in [Type::INT8, Type::INT4_ARRAY, Type::INT8_ARRAY] {
                cache.get_query(&client, sql, &[typ]).await.unwrap();
            }

            // the old query should be re-created
            let q3 = cache.get_query(&client, sql, &types).await.unwrap();
            assert!(
                !Arc::ptr_eq(&q1.metadata, &q3.metadata),
                "q1 and q3 should not re-use the same metadata"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn tracing_lru_cache_reuses_queries_with_different_traceparent() {
        run_with_client(|client| async move {
            let cache = TracingLruCache::with_capacity(1);
            let sql1 = "SELECT $1 /* traceparent=00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01 */";
            let sql2 = "SELECT $1 /* traceparent=00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-02 */";
            let types = [Type::INT4];

            let q1 = cache.get_query(&client, sql1, &types).await.unwrap();
            assert_eq!(q1.sql, sql1);
            let q2 = cache.get_query(&client, sql2, &types).await.unwrap();
            // the requested query traceparent should be preserved
            assert_eq!(q2.sql, sql2);

            assert!(
                Arc::ptr_eq(&q1.metadata, &q2.metadata),
                "q1 and q2 should re-use the same metadata"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn tracing_lru_cache_returns_new_statements_every_time() {
        run_with_client(|client| async move {
            let cache = TracingLruCache::with_capacity(1);
            let sql = "SELECT $1";
            let types = [Type::INT4];

            let q1 = cache.get_statement(&client, sql, &types).await.unwrap();
            let q2 = cache.get_statement(&client, sql, &types).await.unwrap();
            assert_ne!(q1.name(), q2.name());
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
