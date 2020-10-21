use crate::{connect, connection_wrapper::Connection, error::quaint_error_to_connector_error, flavour::SqlFlavour};
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory};
use quaint::prelude::ConnectionInfo;
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use std::path::Path;

#[derive(Debug)]
pub(crate) struct SqliteFlavour {
    pub(super) file_path: String,
    pub(super) attached_name: String,
}

#[async_trait::async_trait]
impl SqlFlavour for SqliteFlavour {
    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        use anyhow::Context;

        let path = Path::new(&self.file_path);
        if path.exists() {
            return Ok(self.file_path.clone());
        }

        let dir = path.parent();

        if let Some((dir, false)) = dir.map(|dir| (dir, dir.exists())) {
            std::fs::create_dir_all(dir)
                .context("Creating SQLite database parent directory.")
                .map_err(|io_err| ConnectorError::generic(io_err))?;
        }

        connect(database_str).await?;

        Ok(self.file_path.clone())
    }

    async fn create_imperative_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE "_prisma_migrations" (
                "id"                    TEXT PRIMARY KEY NOT NULL,
                "checksum"              TEXT NOT NULL,
                "finished_at"           DATETIME,
                "migration_name"        TEXT NOT NULL,
                "logs"                  TEXT NOT NULL,
                "rolled_back_at"        DATETIME,
                "started_at"            DATETIME NOT NULL DEFAULT current_timestamp,
                "applied_steps_count"   INTEGER UNSIGNED NOT NULL DEFAULT 0,
                "script"                TEXT NOT NULL
            );
        "#;

        Ok(connection.raw_cmd(sql).await?)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::sqlite::SqlSchemaDescriber::new(connection.quaint().clone())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    quaint_error_to_connector_error(err, connection.connection_info())
                }
            })
    }

    async fn drop_database(&self, database_url: &str) -> ConnectorResult<()> {
        let file_path = match ConnectionInfo::from_url(database_url) {
            Ok(ConnectionInfo::Sqlite { file_path, .. }) => file_path,
            Ok(_) => unreachable!(),
            Err(err) => return Err(ConnectorError::url_parse_error(err, database_url)),
        };

        std::fs::remove_file(&file_path).map_err(|err| {
            ConnectorError::generic(anyhow::anyhow!(
                "Failed to delete SQLite database at `{}`.\n{}",
                file_path,
                err
            ))
        })?;

        Ok(())
    }

    async fn ensure_connection_validity(&self, _connection: &Connection) -> ConnectorResult<()> {
        Ok(())
    }

    async fn qe_setup(&self, _database_url: &str) -> ConnectorResult<()> {
        use std::fs::File;
        File::create(&self.file_path).expect("Failed to truncate SQLite database");
        Ok(())
    }

    async fn reset(&self, connection: &Connection) -> ConnectorResult<()> {
        let file_path = connection.connection_info().file_path().unwrap();

        std::fs::File::create(file_path).expect("failed to truncate sqlite file");

        Ok(())
    }

    #[tracing::instrument(skip(self, migrations, _connection))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        _connection: &Connection,
    ) -> ConnectorResult<SqlSchema> {
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory.");
        let database_url = format!(
            "file:{}/scratch.db?db_name={}",
            temp_dir.path().to_str().unwrap(),
            self.attached_name
        );

        tracing::debug!("Applying migrations to temporary SQLite database at `{}`", database_url);

        let conn = crate::connect(&database_url).await?;

        for migration in migrations {
            let script = migration.read_migration_script()?;

            tracing::debug!(
                "Applying migration `{}` to temporary database.",
                migration.migration_name()
            );

            conn.raw_cmd(&script)
                .await
                .map_err(ConnectorError::from)
                .map_err(|connector_error| {
                    connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                })?;
        }

        let sql_schema = self.describe_schema(&conn).await?;

        Ok(sql_schema)
    }
}
