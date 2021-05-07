use crate::{sql::TestApi, SchemaAssertion};
use quaint::prelude::Queryable;

pub struct BarrelMigrationExecutor<'a> {
    pub(crate) api: &'a TestApi,
    pub(crate) sql_variant: barrel::backend::SqlVariant,
}

impl BarrelMigrationExecutor<'_> {
    pub async fn execute<F>(self, migration_fn: F) -> SchemaAssertion
    where
        F: FnOnce(&mut barrel::Migration),
    {
        use barrel::Migration;

        let mut migration = if self.api.is_sqlite() {
            Migration::new()
        } else {
            Migration::new().schema(self.api.schema_name())
        };

        migration_fn(&mut migration);

        let full_sql = migration.make_from(self.sql_variant);
        self.api.database().raw_cmd(&full_sql).await.unwrap();

        self.api.assert_schema().await.unwrap()
    }
}
