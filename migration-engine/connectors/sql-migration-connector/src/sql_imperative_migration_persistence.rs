use crate::{error::quaint_error_to_connector_error, SqlMigrationConnector};
use migration_connector::{
    ConnectorResult, FormatChecksum, ImperativeMigrationsPersistence, MigrationRecord, PersistenceNotInitializedError,
};
use quaint::{ast::*, error::ErrorKind as QuaintKind};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const IMPERATIVE_MIGRATIONS_TABLE_NAME: &str = "_prisma_migrations";

#[async_trait::async_trait]
impl ImperativeMigrationsPersistence for SqlMigrationConnector {
    async fn initialize(&self) -> ConnectorResult<()> {
        let mut schema = self.describe_schema().await?;

        // Temporary workaround, for as long as the _Migration table is automatically initialized.
        {
            schema.tables = schema
                .tables
                .drain(..)
                .filter(|table| table.name != "_Migration")
                .collect();

            schema.sequences = schema
                .sequences
                .drain(..)
                .filter(|seq| !seq.name.contains("_Migration"))
                .collect();
        }

        if schema
            .tables
            .iter()
            .any(|table| table.name == IMPERATIVE_MIGRATIONS_TABLE_NAME)
        {
            return Ok(());
        }

        if !schema.is_empty() {
            dbg!(&schema);
            todo!("Non-empty schema")
        }

        self.flavour.create_imperative_migrations_table(&self.conn()).await?;

        Ok(())
    }

    async fn record_migration_started(&self, migration_name: &str, script: &str) -> ConnectorResult<String> {
        let conn = self.conn();
        let id = Uuid::new_v4().to_string();

        let mut hasher = Sha256::new();
        hasher.update(script.as_bytes());
        let checksum: [u8; 32] = hasher.finalize().into();
        let checksum_string = checksum.format_checksum();

        let insert = Insert::single_into((self.schema_name(), IMPERATIVE_MIGRATIONS_TABLE_NAME))
            .value("id", id.as_str())
            .value("checksum", checksum_string.as_str())
            // We need this line because MySQL can't default a text field to an empty string
            .value("logs", "")
            .value("migration_name", migration_name)
            .value("script", script);

        conn.execute(insert).await?;

        Ok(id)
    }

    async fn record_successful_step(&self, id: &str, logs: &str) -> ConnectorResult<()> {
        use quaint::ast::*;

        let update = Update::table((self.schema_name(), IMPERATIVE_MIGRATIONS_TABLE_NAME))
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
        let update = Update::table((self.schema_name(), IMPERATIVE_MIGRATIONS_TABLE_NAME))
            .so_that(Column::from("id").equals(id))
            .set("logs", logs);

        self.conn().execute(update).await?;

        Ok(())
    }

    async fn record_migration_finished(&self, id: &str) -> ConnectorResult<()> {
        let update = Update::table((self.schema_name(), IMPERATIVE_MIGRATIONS_TABLE_NAME))
            .so_that(Column::from("id").equals(id))
            .set("finished_at", chrono::Utc::now()); // TODO maybe use a database generated timestamp

        self.conn().execute(update).await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn list_migrations(&self) -> ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>> {
        let select = Select::from_table((self.schema_name(), IMPERATIVE_MIGRATIONS_TABLE_NAME))
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
            .map_err(|err| quaint_error_to_connector_error(err, self.database_info().connection_info()))?;

        tracing::debug!("Found {} migrations in the migrations table.", rows.len());

        Ok(Ok(rows))
    }
}
