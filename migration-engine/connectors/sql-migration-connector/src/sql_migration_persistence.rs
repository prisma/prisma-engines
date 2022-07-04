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
            let schema = self.flavour.describe_schema().await?;

            if schema
                .table_walkers()
                .any(|table| table.name() == self.flavour().migrations_table_name())
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
        Box::pin(async move {
            let select = Select::from_table(self.flavour().migrations_table())
                .column("id")
                .column("checksum")
                .column("finished_at")
                .column("migration_name")
                .column("logs")
                .column("rolled_back_at")
                .column("started_at")
                .column("applied_steps_count")
                .order_by("started_at".ascend());

            let rows = match self.flavour.query(select.into()).await {
                Ok(result) => result,
                Err(err)
                    if err.is_user_facing_error::<user_facing_errors::query_engine::TableDoesNotExist>()
                        || err.is_user_facing_error::<user_facing_errors::common::InvalidModel>() =>
                {
                    return Ok(Err(PersistenceNotInitializedError))
                }
                err @ Err(_) => err?,
            };

            let rows = rows
                .into_iter()
                .map(|row| -> ConnectorResult<_> {
                    Ok(MigrationRecord {
                        id: row.get("id").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg("Failed to extract `id` from `_prisma_migrations` row.".into())
                        })?,
                        checksum: row.get("checksum").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `checksum` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        finished_at: row.get("finished_at").and_then(|v| v.as_datetime()),
                        migration_name: row.get("migration_name").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `migration_name` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        logs: None,
                        rolled_back_at: row.get("rolled_back_at").and_then(|v| v.as_datetime()),
                        started_at: row.get("started_at").and_then(|v| v.as_datetime()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `started_at` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        applied_steps_count: row.get("applied_steps_count").and_then(|v| v.as_integer()).ok_or_else(
                            || {
                                ConnectorError::from_msg(
                                    "Failed to extract `applied_steps_count` from `_prisma_migrations` row.".into(),
                                )
                            },
                        )? as u32,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            tracing::debug!("Found {} migrations in the migrations table.", rows.len());

            Ok(Ok(rows))
        })
    }
}
