use crate::SqlMigrationConnector;
use migration_connector::{
    BoxFuture, ConnectorError, ConnectorResult, MigrationPersistence, MigrationRecord, PersistenceNotInitializedError,
};
use quaint::ast::*;
use uuid::Uuid;

impl MigrationPersistence for SqlMigrationConnector {
    fn baseline_initialize(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.flavour.create_migrations_table()
    }

    fn initialize(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async move {
            // TODO(MultiSchema): We may need to change this too.
            let schema = self.flavour.describe_schema(None).await?;

            if schema
                .table_walkers()
                .any(|table| table.name() == crate::MIGRATIONS_TABLE_NAME)
            {
                return Ok(());
            }

            if schema
                .table_walkers()
                .any(|t| !self.flavour().table_should_be_ignored(t.name()))
            {
                return Err(ConnectorError::user_facing(
                    user_facing_errors::migration_engine::DatabaseSchemaNotEmpty,
                ));
            }

            self.flavour.create_migrations_table().await?;

            Ok(())
        })
    }

    fn mark_migration_applied_impl<'a>(
        &'a mut self,
        migration_name: &'a str,
        checksum: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<String>> {
        Box::pin(async move {
            let id = Uuid::new_v4().to_string();
            let now = chrono::Utc::now();

            let insert = Insert::single_into(self.flavour().migrations_table())
                .value("id", id.as_str())
                .value("checksum", checksum)
                .value("logs", "")
                .value("started_at", now)
                .value("finished_at", now)
                .value("migration_name", migration_name);

            self.flavour.query(insert.into()).await?;

            Ok(id)
        })
    }

    fn mark_migration_rolled_back_by_id<'a>(&'a mut self, migration_id: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(migration_id))
            .set("rolled_back_at", chrono::Utc::now());

        Box::pin(async move {
            self.flavour.query(update.into()).await?;

            Ok(())
        })
    }

    fn record_migration_started_impl<'a>(
        &'a mut self,
        migration_name: &'a str,
        checksum: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<String>> {
        Box::pin(async move {
            let id = Uuid::new_v4().to_string();
            let now = chrono::Utc::now();

            let insert = Insert::single_into(self.flavour().migrations_table())
                .value("id", id.as_str())
                .value("checksum", checksum)
                .value("started_at", now)
                .value("migration_name", migration_name);

            self.flavour.query(insert.into()).await?;

            Ok(id)
        })
    }

    fn record_successful_step<'a>(&'a mut self, id: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        use quaint::ast::*;

        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(id))
            .set(
                "applied_steps_count",
                Expression::from(Column::from("applied_steps_count")) + Expression::from(1),
            );

        Box::pin(async move {
            self.flavour.query(update.into()).await?;

            Ok(())
        })
    }

    fn record_failed_step<'a>(&'a mut self, id: &'a str, logs: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(id))
            .set("logs", logs);

        Box::pin(async move {
            self.flavour.query(update.into()).await?;

            Ok(())
        })
    }

    fn record_migration_finished<'a>(&'a mut self, id: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(id))
            .set("finished_at", chrono::Utc::now()); // TODO maybe use a database generated timestamp
        Box::pin(async move {
            self.flavour.query(update.into()).await?;

            Ok(())
        })
    }

    #[tracing::instrument(skip(self))]
    fn list_migrations(
        &mut self,
    ) -> BoxFuture<'_, ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>>> {
        self.flavour.load_migrations_table()
    }
}
