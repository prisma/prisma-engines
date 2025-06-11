use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use json_rpc::types::{JsResult, MigrationList};
use psl::SourceFile;
use schema_connector::Migration;

/// A cache for migrations to avoid redundant work during `prisma migrate dev`.
pub struct MigrationCache {
    migrations: HashMap<String, Migration>,
}

impl MigrationCache {
    /// Creates a new cache.
    pub fn new() -> Self {
        Self {
            migrations: Default::default(),
        }
    }

    /// Gets a migration from the cache, or computes it using the provided async closure if not found.
    pub async fn get_or_insert<F, Fut, E>(
        &mut self,
        sources: &Vec<(String, SourceFile)>,
        migrations_list: &MigrationList,
        f: F,
    ) -> Result<&Migration, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Migration, E>>,
    {
        let key = self.cache_key(sources, migrations_list);

        if !self.migrations.contains_key(&key) {
            let migration = f().await?;
            self.migrations.insert(key.clone(), migration);
        }

        Ok(self.migrations.get(&key).unwrap())
    }

    fn cache_key(&self, sources: &Vec<(String, SourceFile)>, migrations_list: &MigrationList) -> String {
        let mut hasher = DefaultHasher::new();

        sources.hash(&mut hasher);

        migrations_list.migration_directories.iter().for_each(|dir| {
            dir.path.hash(&mut hasher);
            dir.migration_file.path.hash(&mut hasher);
            if let JsResult::Ok(content) = &dir.migration_file.content {
                content.hash(&mut hasher);
            }
        });
        migrations_list.lockfile.path.hash(&mut hasher);
        migrations_list.lockfile.content.hash(&mut hasher);

        hasher.finish().to_string()
    }
}

impl Default for MigrationCache {
    fn default() -> Self {
        Self::new()
    }
}
