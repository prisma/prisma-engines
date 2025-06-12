use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use schema_connector::DatabaseSchema;

/// A cache for migrations to avoid redundant work during `prisma migrate dev`.
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

    /// Gets a migration from the cache, or computes it using the provided async closure if not found.
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
        let key = self.cache_key(migration_directories);

        if !self.migrations.contains_key(&key) {
            println!("Cache miss for key: {}", key);
            let schema = f().await?;
            self.migrations.insert(key.clone(), schema);
        } else {
            println!("Cache hit for key: {}", key);
        }

        Ok(self.migrations.get(&key).unwrap().clone())
    }

    fn cache_key<T: Hash>(&self, migration_directories: &Vec<T>) -> String {
        let mut hasher = DefaultHasher::new();

        migration_directories.hash(&mut hasher);

        hasher.finish().to_string()
    }
}

impl Default for MigrationSchemaCache {
    fn default() -> Self {
        Self::new()
    }
}
