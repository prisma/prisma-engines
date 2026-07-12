use std::borrow::Borrow;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use schema_connector::DatabaseSchema;

/// A cache for DatabaseSchemas based on the migration directories to avoid redundant work during `prisma migrate dev`.
#[derive(Default)]
pub struct MigrationSchemaCache {
    migrations: HashMap<String, DatabaseSchema>,
}

impl MigrationSchemaCache {
    /// Creates a new cache.
    pub fn new() -> Self {
        Self {
            migrations: Default::default(),
        }
    }

    /// Gets a DatabaseSchema from the cache, or calls the provided async closure if not found and stores its result in the cache.
    pub async fn get_or_insert<F, Fut, E, T>(&mut self, migration_directories: &[T], f: F) -> Result<DatabaseSchema, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<DatabaseSchema, E>>,
        T: Borrow<json_rpc::types::MigrationDirectory> + Hash,
    {
        let mut hasher = DefaultHasher::new();
        migration_directories.hash(&mut hasher);
        let cache_key = hasher.finish().to_string();

        let entry = self.migrations.entry(cache_key);
        match entry {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let schema = f().await?;
                Ok(entry.insert(schema).clone())
            }
        }
    }
}
