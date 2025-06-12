use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use schema_connector::DatabaseSchema;

/// A cache for DatabaseSchemas based on the migration directories to avoid redundant work during `prisma migrate dev`.
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
    pub async fn get_or_insert<F, Fut, E, T>(
        &mut self,
        migration_directories: &Vec<T>,
        f: F,
    ) -> Result<DatabaseSchema, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<DatabaseSchema, E>>,
        T: Hash,
    {
        let mut hasher = DefaultHasher::new();
        migration_directories.hash(&mut hasher);
        let cache_key = hasher.finish().to_string();

        if !self.migrations.contains_key(&cache_key) {
            let schema = f().await?;
            self.migrations.insert(cache_key.clone(), schema);
        }

        Ok(self.migrations.get(&cache_key).unwrap().clone())
    }
}

impl Default for MigrationSchemaCache {
    fn default() -> Self {
        Self::new()
    }
}
