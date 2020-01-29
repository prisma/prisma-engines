use crate::sql::TestApi;
use quaint::prelude::Queryable;
use sql_schema_describer::SqlSchema;
use std::sync::Arc;

pub struct BarrelMigrationExecutor<'a> {
    pub(crate) api: &'a TestApi,
    pub(crate) sql_variant: barrel::backend::SqlVariant,
}

impl BarrelMigrationExecutor<'_> {
    pub async fn execute<F>(self, migration_fn: F) -> anyhow::Result<SqlSchema>
    where
        F: FnOnce(&mut barrel::Migration) -> (),
    {
        use barrel::Migration;

        let mut migration = Migration::new().schema(self.api.schema_name());
        migration_fn(&mut migration);

        let full_sql = migration.make_from(self.sql_variant);
        run_full_sql(&self.api.database(), &full_sql).await?;

        let mut result = self.api.describe_database().await.expect("Description failed");

        // The presence of the _Migration table makes assertions harder. Therefore remove it.
        result.tables = result.tables.into_iter().filter(|t| t.name != "_Migration").collect();

        Ok(result)
    }
}

async fn run_full_sql(database: &Arc<dyn Queryable + Send + Sync>, full_sql: &str) -> anyhow::Result<()> {
    for sql in full_sql.split(";").filter(|sql| !sql.is_empty()) {
        database.query_raw(&sql, &[]).await?;
    }

    Ok(())
}
