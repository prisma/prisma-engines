use super::SqlFlavour;
use crate::{catch, connect, SqlError, SqlResult};
use futures::TryFutureExt;
use migration_connector::{ConnectorError, ConnectorResult, ErrorKind, MigrationDirectory};
use quaint::{prelude::ConnectionInfo, prelude::Queryable, prelude::SqlFamily, single::Quaint};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::path::Path;

#[derive(Debug)]
pub(crate) struct SqliteFlavour {
    pub(super) file_path: String,
    pub(super) attached_name: String,
}

impl SqliteFlavour {
    pub(crate) fn attached_name(&self) -> &str {
        &self.attached_name
    }
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
                .map_err(|io_err| ConnectorError::from_kind(migration_connector::ErrorKind::Generic(io_err)))?;
        }

        connect(database_str).await?;

        Ok(self.file_path.clone())
    }

    async fn describe_schema<'a>(&'a self, schema_name: &'a str, conn: Quaint) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::sqlite::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn ensure_connection_validity(&self, _connection: &Quaint) -> ConnectorResult<()> {
        Ok(())
    }

    async fn ensure_imperative_migrations_table(
        &self,
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<()> {
        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS "{}"."_prisma_migrations" (
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
        "#,
            self.attached_name()
        );

        catch(connection_info, connection.raw_cmd(&sql).map_err(SqlError::from)).await
    }

    async fn qe_setup(&self, _database_url: &str) -> ConnectorResult<()> {
        use std::fs::File;
        File::create(&self.file_path).expect("Failed to truncate SQLite database");
        Ok(())
    }

    async fn reset(&self, conn: &dyn Queryable, connection_info: &ConnectionInfo) -> ConnectorResult<()> {
        let file_path = connection_info.file_path().unwrap();

        std::fs::remove_file(file_path).map_err(|err| {
            ConnectorError::from_kind(ErrorKind::Generic(anyhow::anyhow!(
                "Failed to delete SQLite database at `{}`. {}",
                file_path,
                err
            )))
        })?;

        catch(
            connection_info,
            conn.execute_raw("DETACH ?", &[connection_info.schema_name().into()])
                .map_err(SqlError::from),
        )
        .await?;

        catch(
            connection_info,
            conn.execute_raw(
                "ATTACH DATABASE ? AS ?",
                &[file_path.into(), connection_info.schema_name().into()],
            )
            .map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Sqlite
    }

    #[tracing::instrument(skip(self, migrations, _connection, _connection_info))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        _connection: &dyn Queryable,
        _connection_info: &ConnectionInfo,
    ) -> ConnectorResult<SqlSchema> {
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory.");
        let database_url = format!(
            "file:{}/scratch.db?db_name={}",
            temp_dir.path().to_str().unwrap(),
            self.attached_name
        );

        tracing::debug!("Applying migrations to temporary SQLite database at `{}`", database_url);

        let connection_info = ConnectionInfo::from_url(&database_url)
            .map_err(|err| ConnectorError::url_parse_error(err, &database_url))?;
        let conn = catch(&connection_info, Quaint::new(&database_url).map_err(SqlError::from)).await?;

        for migration in migrations {
            let script = migration
                .read_migration_script()
                .expect("failed to read migration script");

            tracing::debug!(
                "Applying migration `{}` to temporary database.",
                migration.migration_name()
            );

            catch(conn.connection_info(), conn.raw_cmd(&script).map_err(SqlError::from))
                .await
                .map_err(|connector_error| {
                    connector_error.into_migration_failed(migration.migration_name().to_owned())
                })?;
        }

        let sql_schema = catch(
            &conn.connection_info().clone(),
            self.describe_schema(&self.attached_name, conn),
        )
        .await?;

        Ok(sql_schema)
    }
}
