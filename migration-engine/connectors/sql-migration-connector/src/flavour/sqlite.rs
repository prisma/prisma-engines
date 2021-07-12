use crate::{
    connect, connection_wrapper::Connection, error::quaint_error_to_connector_error, flavour::SqlFlavour,
    SqlMigrationConnector,
};
use datamodel::common::preview_features::PreviewFeature;
use enumflags2::BitFlags;
use indoc::indoc;
use migration_connector::{migrations_directory::MigrationDirectory, ConnectorError, ConnectorResult};
use quaint::prelude::ConnectionInfo;
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use std::path::Path;

#[derive(Debug)]
pub(crate) struct SqliteFlavour {
    pub(super) file_path: String,
    pub(super) attached_name: String,
    pub(super) preview_features: BitFlags<PreviewFeature>,
}

#[async_trait::async_trait]
impl SqlFlavour for SqliteFlavour {
    async fn acquire_lock(&self, connection: &Connection) -> ConnectorResult<()> {
        connection.raw_cmd("PRAGMA main.locking_mode=EXCLUSIVE").await?;

        Ok(())
    }

    async fn run_query_script(&self, sql: &str, connection: &Connection) -> ConnectorResult<()> {
        Ok(connection.raw_cmd(sql).await?)
    }

    async fn apply_migration_script(
        &self,
        migration_name: &str,
        script: &str,
        conn: &Connection,
    ) -> ConnectorResult<()> {
        super::generic_apply_migration_script(migration_name, script, conn).await
    }

    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let path = Path::new(&self.file_path);

        if path.exists() {
            return Ok(self.file_path.clone());
        }

        let dir = path.parent();

        if let Some((dir, false)) = dir.map(|dir| (dir, dir.exists())) {
            std::fs::create_dir_all(dir)
                .map_err(|err| ConnectorError::from_source(err, "Creating SQLite database parent directory."))?;
        }

        connect(database_str).await?;

        Ok(self.file_path.clone())
    }

    async fn create_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = indoc! {r#"
            CREATE TABLE "_prisma_migrations" (
                "id"                    TEXT PRIMARY KEY NOT NULL,
                "checksum"              TEXT NOT NULL,
                "finished_at"           DATETIME,
                "migration_name"        TEXT NOT NULL,
                "logs"                  TEXT,
                "rolled_back_at"        DATETIME,
                "started_at"            DATETIME NOT NULL DEFAULT current_timestamp,
                "applied_steps_count"   INTEGER UNSIGNED NOT NULL DEFAULT 0
            );
        "#};

        Ok(connection.raw_cmd(sql).await?)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::sqlite::SqlSchemaDescriber::new(connection.queryable())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    quaint_error_to_connector_error(err, &connection.connection_info())
                }
                DescriberErrorKind::CrossSchemaReference { .. } => {
                    unreachable!("No schemas in SQLite")
                }
            })
    }

    async fn drop_database(&self, database_url: &str) -> ConnectorResult<()> {
        let file_path = match ConnectionInfo::from_url(database_url) {
            Ok(ConnectionInfo::Sqlite { file_path, .. }) => file_path,
            Ok(_) => unreachable!(),
            Err(err) => return Err(ConnectorError::url_parse_error(err)),
        };

        std::fs::remove_file(&file_path).map_err(|err| {
            ConnectorError::from_msg(format!("Failed to delete SQLite database at `{}`.\n{}", file_path, err))
        })?;

        Ok(())
    }

    async fn drop_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        connection.raw_cmd("DROP TABLE _prisma_migrations").await?;

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
        let connection_info = connection.connection_info();
        let file_path = connection_info.file_path().unwrap();

        connection.raw_cmd("PRAGMA main.locking_mode=NORMAL").await?;
        connection.raw_cmd("PRAGMA main.quick_check").await?;

        tracing::debug!("Truncating {:?}", file_path);
        std::fs::File::create(file_path).expect("failed to truncate sqlite file");

        self.acquire_lock(connection).await?;

        Ok(())
    }

    #[tracing::instrument(skip(self, migrations, _connection, _connector))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        _connection: &Connection,
        _connector: &SqlMigrationConnector,
    ) -> ConnectorResult<SqlSchema> {
        tracing::debug!("Applying migrations to temporary in-memory SQLite database.");
        let quaint = quaint::single::Quaint::new_in_memory().map_err(|err| {
            quaint_error_to_connector_error(
                err,
                &ConnectionInfo::InMemorySqlite {
                    db_name: self.attached_name.clone(),
                },
            )
        })?;
        let conn = Connection::new_generic(quaint);

        for migration in migrations {
            let script = migration.read_migration_script()?;

            tracing::debug!(
                "Applying migration `{}` to shadow database.",
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

    fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.preview_features
    }
}
