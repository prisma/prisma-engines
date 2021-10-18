use crate::SqlMigrationConnector;
use migration_connector::{
    ConnectorError, ConnectorResult, MigrationPersistence, MigrationRecord, PersistenceNotInitializedError,
};
use quaint::ast::*;
use uuid::Uuid;

#[async_trait::async_trait]
impl MigrationPersistence for SqlMigrationConnector {
    async fn baseline_initialize(&self) -> ConnectorResult<()> {
        let conn = self.conn().await?;
        self.flavour.create_migrations_table(conn).await?;

        Ok(())
    }

    async fn initialize(&self) -> ConnectorResult<()> {
        let schema = self.describe_schema().await?;

        if schema
            .tables
            .iter()
            .any(|table| table.name == self.flavour().migrations_table_name())
        {
            return Ok(());
        }

        if !schema.is_empty()
            && schema
                .table_walkers()
                .any(|t| !self.flavour().table_should_be_ignored(t.name()))
        {
            return Err(ConnectorError::user_facing(
                user_facing_errors::migration_engine::DatabaseSchemaNotEmpty {
                    database_name: self.connection_info.database_location(),
                },
            ));
        }

        let conn = self.conn().await?;
        self.flavour.create_migrations_table(conn).await?;

        Ok(())
    }

    async fn mark_migration_applied_impl(&self, migration_name: &str, checksum: &str) -> ConnectorResult<String> {
        let conn = self.conn().await?;
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let insert = Insert::single_into(self.flavour().migrations_table())
            .value("id", id.as_str())
            .value("checksum", checksum)
            .value("logs", "")
            .value("started_at", now)
            .value("finished_at", now)
            .value("migration_name", migration_name);

        conn.query(insert).await?;

        Ok(id)
    }

    async fn mark_migration_rolled_back_by_id(&self, migration_id: &str) -> ConnectorResult<()> {
        let conn = self.conn().await?;

        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(migration_id))
            .set("rolled_back_at", chrono::Utc::now());

        conn.query(update).await?;

        Ok(())
    }

    async fn record_migration_started_impl(&self, migration_name: &str, checksum: &str) -> ConnectorResult<String> {
        let conn = self.conn().await?;
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let insert = Insert::single_into(self.flavour().migrations_table())
            .value("id", id.as_str())
            .value("checksum", checksum)
            .value("started_at", now)
            .value("migration_name", migration_name);

        conn.query(insert).await?;

        Ok(id)
    }

    async fn record_successful_step(&self, id: &str) -> ConnectorResult<()> {
        use quaint::ast::*;

        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(id))
            .set(
                "applied_steps_count",
                Expression::from(Column::from("applied_steps_count")) + Expression::from(1),
            );

        self.conn().await?.query(update).await?;

        Ok(())
    }

    async fn record_failed_step(&self, id: &str, logs: &str) -> ConnectorResult<()> {
        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(id))
            .set("logs", logs);

        self.conn().await?.query(update).await?;

        Ok(())
    }

    async fn record_migration_finished(&self, id: &str) -> ConnectorResult<()> {
        let update = Update::table(self.flavour().migrations_table())
            .so_that(Column::from("id").equals(id))
            .set("finished_at", chrono::Utc::now()); // TODO maybe use a database generated timestamp

        self.conn().await?.query(update).await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn list_migrations(&self) -> ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>> {
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

        let rows = match self.conn().await?.query(select).await {
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
                        ConnectorError::from_msg("Failed to extract `checksum` from `_prisma_migrations` row.".into())
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
                        ConnectorError::from_msg("Failed to extract `started_at` from `_prisma_migrations` row.".into())
                    })?,
                    applied_steps_count: row.get("applied_steps_count").and_then(|v| v.as_i64()).ok_or_else(|| {
                        ConnectorError::from_msg(
                            "Failed to extract `applied_steps_count` from `_prisma_migrations` row.".into(),
                        )
                    })? as u32,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        tracing::debug!("Found {} migrations in the migrations table.", rows.len());

        Ok(Ok(rows))
    }
}
