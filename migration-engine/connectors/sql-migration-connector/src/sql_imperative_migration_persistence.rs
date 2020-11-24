use crate::{error::quaint_error_to_connector_error, SqlMigrationConnector};
use migration_connector::{
    ConnectorError, ConnectorResult, ImperativeMigrationsPersistence, MigrationRecord, PersistenceNotInitializedError,
};
use quaint::{ast::*, error::ErrorKind as QuaintKind};
use uuid::Uuid;

const IMPERATIVE_MIGRATIONS_TABLE_NAME: &str = "_prisma_migrations";

#[async_trait::async_trait]
impl ImperativeMigrationsPersistence for SqlMigrationConnector {
    async fn baseline_initialize(&self) -> ConnectorResult<()> {
        self.flavour.create_imperative_migrations_table(&self.conn()).await?;

        Ok(())
    }

    async fn initialize(&self) -> ConnectorResult<()> {
        let schema = self.describe_schema().await?;

        if schema
            .tables
            .iter()
            .any(|table| table.name == IMPERATIVE_MIGRATIONS_TABLE_NAME)
        {
            return Ok(());
        }

        if !schema.is_empty() {
            return Err(ConnectorError::user_facing_error(
                user_facing_errors::migration_engine::DatabaseSchemaNotEmpty {
                    database_name: self.connection.connection_info().database_location().to_owned(),
                },
            ));
        }

        self.flavour.create_imperative_migrations_table(&self.conn()).await?;

        Ok(())
    }

    async fn mark_migration_applied_impl(
        &self,
        migration_name: &str,
        script: &str,
        checksum: &str,
    ) -> ConnectorResult<String> {
        let conn = self.conn();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let insert = Insert::single_into(IMPERATIVE_MIGRATIONS_TABLE_NAME)
            .value("id", id.as_str())
            .value("checksum", checksum)
            .value("logs", "")
            .value("started_at", now)
            .value("finished_at", now)
            .value("migration_name", migration_name)
            .value("script", script);

        conn.execute(insert).await?;

        Ok(id)
    }

    async fn mark_migration_rolled_back_by_id(&self, migration_id: &str) -> ConnectorResult<()> {
        let conn = self.conn();

        let update = Update::table(IMPERATIVE_MIGRATIONS_TABLE_NAME)
            .so_that(Column::from("id").equals(migration_id))
            .set("rolled_back_at", chrono::Utc::now());

        conn.execute(update).await?;

        Ok(())
    }

    async fn record_migration_started_impl(
        &self,
        migration_name: &str,
        script: &str,
        checksum: &str,
    ) -> ConnectorResult<String> {
        let conn = self.conn();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let insert = Insert::single_into(IMPERATIVE_MIGRATIONS_TABLE_NAME)
            .value("id", id.as_str())
            .value("checksum", checksum)
            .value("started_at", now)
            // We need this line because MySQL can't default a text field to an empty string
            .value("logs", "")
            .value("migration_name", migration_name)
            .value("script", script);

        conn.execute(insert).await?;

        Ok(id)
    }

    async fn record_successful_step(&self, id: &str, logs: &str) -> ConnectorResult<()> {
        use quaint::ast::*;

        let update = Update::table(IMPERATIVE_MIGRATIONS_TABLE_NAME)
            .so_that(Column::from("id").equals(id))
            .set(
                "applied_steps_count",
                Expression::from(Column::from("applied_steps_count")) + Expression::from(1),
            )
            .set("logs", logs);

        self.conn().execute(update).await?;

        Ok(())
    }

    async fn record_failed_step(&self, id: &str, logs: &str) -> ConnectorResult<()> {
        let update = Update::table(IMPERATIVE_MIGRATIONS_TABLE_NAME)
            .so_that(Column::from("id").equals(id))
            .set("logs", logs);

        self.conn().execute(update).await?;

        Ok(())
    }

    async fn record_migration_finished(&self, id: &str) -> ConnectorResult<()> {
        let update = Update::table(IMPERATIVE_MIGRATIONS_TABLE_NAME)
            .so_that(Column::from("id").equals(id))
            .set("finished_at", chrono::Utc::now()); // TODO maybe use a database generated timestamp

        self.conn().execute(update).await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn list_migrations(&self) -> ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>> {
        let select = Select::from_table(IMPERATIVE_MIGRATIONS_TABLE_NAME)
            .column("id")
            .column("checksum")
            .column("finished_at")
            .column("migration_name")
            .column("logs")
            .column("rolled_back_at")
            .column("started_at")
            .column("applied_steps_count")
            .column("script")
            .order_by("started_at".ascend());

        let result = match self.conn().query(select).await {
            Ok(result) => result,
            Err(err) if matches!(err.kind(), QuaintKind::TableDoesNotExist { table } if table.contains(IMPERATIVE_MIGRATIONS_TABLE_NAME)) => {
                return Ok(Err(PersistenceNotInitializedError))
            }
            err @ Err(_) => err?,
        };

        let rows = quaint::serde::from_rows(result)
            .map_err(|err| quaint_error_to_connector_error(err, self.connection.connection_info()))?;

        tracing::debug!("Found {} migrations in the migrations table.", rows.len());

        Ok(Ok(rows))
    }
}
