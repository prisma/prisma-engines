use std::hash::{BuildHasher, Hash, RandomState};

use async_trait::async_trait;
use futures::lock::Mutex;
use lru_cache::LruCache;
use postgres_types::Type;
use tokio_postgres::{Client, Error, Statement};

use crate::connector::metrics::strip_query_traceparent;

use super::query::{IsQuery, TypedQuery};

#[async_trait]
pub trait QueryCache: From<CacheSettings> + Send + Sync {
    type Query: IsQuery;

    async fn get_by_query(&self, client: &Client, sql: &str, types: &[Type]) -> Result<Self::Query, Error>;
}

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
        Self::default()
    }
}

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

#[derive(Debug)]
pub struct LruTracingCache {
    cache: InnerLruCache<TypedQuery>,
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
    type Query = TypedQuery;

    async fn get_by_query(&self, client: &Client, sql: &str, types: &[Type]) -> Result<TypedQuery, Error> {
        let sql_without_traceparent = strip_query_traceparent(sql);

        match self.cache.get(sql_without_traceparent, types).await {
            Some(query) => Ok(query),
            None => {
                let stmt = client.prepare_typed(sql, types).await?;
                let query = TypedQuery {
                    sql: sql.into(),
                    types: stmt.columns().iter().map(|c| c.type_().clone()).collect(),
                };
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

#[derive(Debug)]
pub struct CacheSettings {
    pub capacity: usize,
}

/// Key uniquely representing an SQL statement in the prepared statements cache.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct QueryKey {
    /// Hash of a string with SQL query.
    sql: u64,
    /// Combined hash of types for all parameters from the query.
    types_hash: u64,
}

impl QueryKey {
    fn new(sql: &str, params: &[Type]) -> Self {
        let st = RandomState::new();
        Self {
            sql: st.hash_one(sql),
            types_hash: st.hash_one(params),
        }
    }
}

#[derive(Debug)]
struct InnerLruCache<V> {
    cache: Mutex<LruCache<QueryKey, V>>,
}

impl<V> InnerLruCache<V> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
        }
    }

    async fn get(&self, sql: &str, types: &[Type]) -> Option<V>
    where
        V: Clone,
    {
        let mut cache = self.cache.lock().await;
        let capacity = cache.capacity();
        let stored = cache.len();

        let key = QueryKey::new(sql, types);
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
        self.cache.lock().await.insert(QueryKey::new(sql, types), value);
    }
}
