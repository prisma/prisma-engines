use super::SqlFlavour;
use crate::{
    connect, connection_wrapper::Connection, database_info::DatabaseInfo, error::CheckDatabaseInfoResult,
    error::SystemDatabase, SqlError,
};
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory};
use once_cell::sync::Lazy;
use quaint::{connector::MysqlUrl, prelude::SqlFamily};
use regex::RegexSet;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use url::Url;

#[derive(Debug)]
pub(crate) struct MysqlFlavour(pub(super) MysqlUrl);

impl MysqlFlavour {
    pub(crate) fn schema_name(&self) -> &str {
        self.0.dbname()
    }
}

#[async_trait::async_trait]
impl SqlFlavour for MysqlFlavour {
    fn check_database_info(&self, database_info: &DatabaseInfo) -> CheckDatabaseInfoResult {
        static MYSQL_SYSTEM_DATABASES: Lazy<regex::RegexSet> = Lazy::new(|| {
            RegexSet::new(&[
                "(?i)^mysql$",
                "(?i)^information_schema$",
                "(?i)^performance_schema$",
                "(?i)^sys$",
            ])
            .unwrap()
        });

        let db_name = database_info.connection_info().schema_name();

        if MYSQL_SYSTEM_DATABASES.is_match(db_name) {
            return Err(SystemDatabase(db_name.to_owned()));
        }

        Ok(())
    }

    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        url.set_path("/mysql");

        let conn = connect(&url.to_string()).await?;
        let db_name = self.0.dbname();

        let query = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
            db_name
        );

        conn.raw_cmd(&query).await?;

        Ok(db_name.to_owned())
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::mysql::SqlSchemaDescriber::new(connection.quaint().clone())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(SqlError::from)
            .map_err(|err| err.into_connector_error(connection.connection_info()))
    }

    async fn ensure_connection_validity(&self, connection: &Connection) -> ConnectorResult<()> {
        connection.raw_cmd("SELECT 1").await?;

        Ok(())
    }

    async fn ensure_imperative_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS _prisma_migrations (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             DATETIME(3),
                migration_name          TEXT NOT NULL,
                logs                    TEXT NOT NULL,
                rolled_back_at          DATETIME(3),
                started_at              DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
                applied_steps_count     INTEGER UNSIGNED NOT NULL DEFAULT 0,
                script                  TEXT NOT NULL
            ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
        "#;

        connection.raw_cmd(sql).await
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        url.set_path("/mysql");

        let conn = connect(&url.to_string()).await?;
        let db_name = self.0.dbname();

        let query = format!("DROP DATABASE IF EXISTS `{}`", db_name);
        conn.raw_cmd(&query).await?;

        let query = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
            db_name
        );
        conn.raw_cmd(&query).await?;

        Ok(())
    }

    async fn reset(&self, connection: &Connection) -> ConnectorResult<()> {
        let db_name = connection.connection_info().dbname().unwrap();

        connection.raw_cmd(&format!("DROP DATABASE `{}`", db_name)).await?;
        connection.raw_cmd(&format!("CREATE DATABASE `{}`", db_name)).await?;
        connection.raw_cmd(&format!("USE `{}`", db_name)).await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mysql
    }

    #[tracing::instrument(skip(self, migrations, connection))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
    ) -> ConnectorResult<SqlSchema> {
        let database_name = format!("prisma_shadow_db{}", uuid::Uuid::new_v4());
        let drop_database = format!("DROP DATABASE IF EXISTS `{}`", database_name);
        let create_database = format!("CREATE DATABASE `{}`", database_name);

        connection.raw_cmd(&drop_database).await?;
        connection.raw_cmd(&create_database).await?;

        let mut temporary_database_url = self.0.url().clone();
        temporary_database_url.set_path(&format!("/{}", database_name));
        let temporary_database_url = temporary_database_url.to_string();

        tracing::debug!("Connecting to temporary database at {:?}", temporary_database_url);

        let temp_database = crate::connect(&temporary_database_url).await?;

        for migration in migrations {
            let script = migration
                .read_migration_script()
                .expect("failed to read migration script");

            tracing::debug!(
                "Applying migration `{}` to temporary database.",
                migration.migration_name()
            );

            temp_database.raw_cmd(&script).await.map_err(|connector_error| {
                connector_error.into_migration_failed(migration.migration_name().to_owned())
            })?;
        }

        let sql_schema = self.describe_schema(&temp_database).await?;

        connection.raw_cmd(&drop_database).await?;

        Ok(sql_schema)
    }
}
