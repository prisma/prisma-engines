use crate::sql::TestApi;
use quaint::prelude::Queryable;
use sql_schema_describer::SqlSchema;

pub struct BarrelMigrationExecutor<'a> {
    pub(crate) api: &'a TestApi,
    pub(crate) sql_variant: barrel::backend::SqlVariant,
}

impl BarrelMigrationExecutor<'_> {
    pub async fn execute<F>(self, migration_fn: F) -> anyhow::Result<SqlSchema>
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

        let result = self.api.describe_database().await.expect("Description failed");

        Ok(result)
    }
}
