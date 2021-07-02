use crate::{
    connect, connection_wrapper::Connection, error::quaint_error_to_connector_error, SqlFlavour, SqlMigrationConnector,
};
use connection_string::JdbcString;
use datamodel::common::preview_features::PreviewFeature;
use enumflags2::BitFlags;
use indoc::formatdoc;
use migration_connector::{migrations_directory::MigrationDirectory, ConnectorError, ConnectorResult};
use quaint::{connector::MssqlUrl, prelude::Table};
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use std::str::FromStr;
use user_facing_errors::{introspection_engine::DatabaseSchemaInconsistent, KnownError};

pub(crate) struct MssqlFlavour {
    url: MssqlUrl,
    preview_features: BitFlags<PreviewFeature>,
}

impl std::fmt::Debug for MssqlFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MssqlFlavour").field("url", &"<REDACTED>").finish()
    }
}

impl MssqlFlavour {
    pub fn new(url: MssqlUrl, preview_features: BitFlags<PreviewFeature>) -> Self {
        Self { url, preview_features }
    }

    fn is_running_on_azure_sql(&self) -> bool {
        self.url.host().contains(".database.windows.net")
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.url.schema()
    }

    /// Get the url as a JDBC string, extract the database name, and re-encode the string.
    fn master_url(input: &str) -> ConnectorResult<(String, String)> {
        let mut conn = JdbcString::from_str(&format!("jdbc:{}", input))
            .map_err(|e| ConnectorError::from_source(e, "JDBC string parse error"))?;
        let params = conn.properties_mut();

        let db_name = params.remove("database").unwrap_or_else(|| String::from("master"));
        Ok((db_name, conn.to_string()))
    }

    async fn clean_up_shadow_database(&self, main_connection: &Connection, database_name: &str) -> ConnectorResult<()> {
        let drop_database = format!("DROP DATABASE [{}]", database = database_name);
        main_connection.raw_cmd(&drop_database).await?;

        Ok(())
    }

    /// Returns a connection, and maybe a database name to clean up.
    async fn shadow_database_connection(
        &self,
        main_connection: &Connection,
        connector: &SqlMigrationConnector,
        shadow_database_name: Option<&str>,
    ) -> ConnectorResult<Connection> {
        if let Some(shadow_database_connection_string) = &connector.shadow_database_connection_string {
            let conn = crate::connect(shadow_database_connection_string).await?;

            let shadow_conninfo = conn.connection_info();
            let main_conninfo = main_connection.connection_info();

            super::validate_connection_infos_do_not_match((&shadow_conninfo, &main_conninfo))?;

            if self.reset(&conn).await.is_err() {
                connector.best_effort_reset(&conn).await?;
            }

            return Ok(conn);
        }

        // See https://github.com/prisma/prisma/issues/6371 for the rationale on
        // this conditional.
        if self.is_running_on_azure_sql() {
            return Err(ConnectorError::user_facing(
                user_facing_errors::migration_engine::AzureMssqlShadowDb,
            ));
        }

        let database_name = shadow_database_name.unwrap();
        let create_database = format!("CREATE DATABASE [{}]", database_name);

        main_connection
            .raw_cmd(&create_database)
            .await
            .map_err(ConnectorError::from)
            .map_err(|err| err.into_shadow_db_creation_error())?;

        let mut jdbc_string: JdbcString = self.url.connection_string().parse().unwrap();
        jdbc_string
            .properties_mut()
            .insert("database".into(), database_name.into());
        let host = jdbc_string.server_name();

        let jdbc_string = jdbc_string.to_string();

        tracing::debug!("Connecting to shadow database at {}", host.unwrap_or("localhost"));

        Ok(crate::connect(&jdbc_string).await?)
    }
}

#[async_trait::async_trait]
impl SqlFlavour for MssqlFlavour {
    async fn acquire_lock(&self, connection: &Connection) -> ConnectorResult<()> {
        // see
        // https://docs.microsoft.com/en-us/sql/relational-databases/system-stored-procedures/sp-getapplock-transact-sql?view=sql-server-ver15
        // We don't set an explicit timeout because we want to respect the
        // server-set default.
        Ok(connection
            .raw_cmd("sp_getapplock @Resource = 'prisma_migrate', @LockMode = 'Exclusive', @LockOwner = 'Session'")
            .await?)
    }

    async fn apply_migration_script(
        &self,
        migration_name: &str,
        script: &str,
        conn: &Connection,
    ) -> ConnectorResult<()> {
        super::generic_apply_migration_script(migration_name, script, conn).await
    }

    fn migrations_table(&self) -> Table<'_> {
        (self.schema_name(), self.migrations_table_name()).into()
    }

    fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.preview_features
    }

    async fn create_database(&self, jdbc_string: &str) -> ConnectorResult<String> {
        let (db_name, master_uri) = Self::master_url(jdbc_string)?;
        let conn = connect(&master_uri.to_string()).await?;

        let query = format!("CREATE DATABASE [{}]", db_name);
        conn.raw_cmd(&query).await?;

        let conn = connect(jdbc_string).await?;

        let query = format!("CREATE SCHEMA {}", conn.connection_info().schema_name(),);

        conn.raw_cmd(&query).await?;

        Ok(db_name)
    }

    async fn create_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = formatdoc! { r#"
            CREATE TABLE [{}].[{}] (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             DATETIMEOFFSET,
                migration_name          NVARCHAR(250) NOT NULL,
                logs                    NVARCHAR(MAX) NULL,
                rolled_back_at          DATETIMEOFFSET,
                started_at              DATETIMEOFFSET NOT NULL DEFAULT CURRENT_TIMESTAMP,
                applied_steps_count     INT NOT NULL DEFAULT 0
            );
        "#, self.schema_name(), self.migrations_table_name()};

        Ok(connection.raw_cmd(&sql).await?)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::mssql::SqlSchemaDescriber::new(connection.queryable())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    quaint_error_to_connector_error(err, &connection.connection_info())
                }
                e @ DescriberErrorKind::CrossSchemaReference { .. } => {
                    let err = KnownError::new(DatabaseSchemaInconsistent {
                        explanation: format!("{}", e),
                    });

                    ConnectorError::from(err)
                }
            })
    }

    async fn drop_database(&self, _database_url: &str) -> ConnectorResult<()> {
        let features = vec!["microsoftSqlServer".into()];
        return Err(ConnectorError::user_facing(
            user_facing_errors::migration_engine::PreviewFeaturesBlocked { features },
        ));
    }

    async fn drop_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        connection
            .raw_cmd(&format!(
                "DROP TABLE [{}].[{}]",
                self.schema_name(),
                self.migrations_table_name()
            ))
            .await?;

        Ok(())
    }

    async fn run_query_script(&self, sql: &str, connection: &Connection) -> ConnectorResult<()> {
        Ok(connection.raw_cmd(sql).await?)
    }

    async fn reset(&self, connection: &Connection) -> ConnectorResult<()> {
        let schema_name = connection.schema_name();

        let drop_procedures = format!(
            r#"
            DECLARE @stmt NVARCHAR(max)
            DECLARE @n CHAR(1)

            SET @n = CHAR(10)

            SELECT @stmt = ISNULL(@stmt + @n, '') +
                'DROP PROCEDURE [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(object_id) + ']'
            FROM sys.objects
            WHERE SCHEMA_NAME(schema_id) = '{0}' AND type = 'P'

            EXEC SP_EXECUTESQL @stmt
            "#,
            schema_name
        );

        let drop_shared_defaults = format!(
            r#"
            DECLARE @stmt NVARCHAR(max)
            DECLARE @n CHAR(1)

            SET @n = CHAR(10)

            SELECT @stmt = ISNULL(@stmt + @n, '') +
                'DROP DEFAULT [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(object_id) + ']'
            FROM sys.objects
            WHERE SCHEMA_NAME(schema_id) = '{0}' AND type = 'D' AND parent_object_id = 0

            EXEC SP_EXECUTESQL @stmt
            "#,
            schema_name
        );

        let drop_views = format!(
            r#"
            DECLARE @stmt NVARCHAR(max)
            DECLARE @n CHAR(1)

            SET @n = CHAR(10)

            SELECT @stmt = ISNULL(@stmt + @n, '') +
                'DROP VIEW [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
            FROM sys.views
            WHERE SCHEMA_NAME(schema_id) = '{0}'

            EXEC SP_EXECUTESQL @stmt
            "#,
            schema_name
        );

        let drop_fks = format!(
            r#"
            DECLARE @stmt NVARCHAR(max)
            DECLARE @n CHAR(1)

            SET @n = CHAR(10)

            SELECT @stmt = ISNULL(@stmt + @n, '') +
                'ALTER TABLE [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(parent_object_id) + '] DROP CONSTRAINT [' + name + ']'
            FROM sys.foreign_keys
            WHERE SCHEMA_NAME(schema_id) = '{0}'

            EXEC SP_EXECUTESQL @stmt
            "#,
            schema_name
        );

        let drop_tables = format!(
            r#"
            DECLARE @stmt NVARCHAR(max)
            DECLARE @n CHAR(1)

            SET @n = CHAR(10)

            SELECT @stmt = ISNULL(@stmt + @n, '') +
                'DROP TABLE [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
            FROM sys.tables
            WHERE SCHEMA_NAME(schema_id) = '{0}'

            EXEC SP_EXECUTESQL @stmt
            "#,
            schema_name
        );

        connection.raw_cmd(&drop_procedures).await?;
        connection.raw_cmd(&drop_views).await?;
        connection.raw_cmd(&drop_shared_defaults).await?;
        connection.raw_cmd(&drop_fks).await?;
        connection.raw_cmd(&drop_tables).await?;

        Ok(())
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let (db_name, master_uri) = Self::master_url(database_str)?;
        let conn = connect(&master_uri).await?;

        // Without these, our poor connection gets deadlocks if other schemas
        // are modified while we introspect.
        let allow_snapshot_isolation = format!(
            "ALTER DATABASE [{db_name}] SET ALLOW_SNAPSHOT_ISOLATION ON",
            db_name = db_name
        );

        conn.raw_cmd(&allow_snapshot_isolation).await.unwrap();

        self.reset(&conn).await?;

        conn.raw_cmd(&format!(
            "DROP SCHEMA IF EXISTS {}",
            conn.connection_info().schema_name()
        ))
        .await?;

        conn.raw_cmd(&format!("CREATE SCHEMA {}", conn.connection_info().schema_name(),))
            .await
            .unwrap();

        Ok(())
    }

    async fn ensure_connection_validity(&self, connection: &Connection) -> ConnectorResult<()> {
        connection.raw_cmd("SELECT 1").await?;

        Ok(())
    }

    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
        connector: &SqlMigrationConnector,
    ) -> ConnectorResult<SqlSchema> {
        let shadow_database_name = connector.shadow_database_name();

        // We must create the connection in a block, closing it before dropping
        // the database.
        let sql_schema_result = {
            // We go through the whole process without early return, then clean up
            // the shadow database, and only then return the result. This avoids
            // leaving shadow databases behind in case of e.g. faulty
            // migrations.

            let temp_database = self
                .shadow_database_connection(connection, connector, shadow_database_name.as_deref())
                .await?;

            if self.schema_name() != "dbo" {
                let create_schema = format!("CREATE SCHEMA [{schema}]", schema = self.schema_name());

                temp_database.raw_cmd(&create_schema).await?;
            }

            (|| async {
                for migration in migrations {
                    let script = migration.read_migration_script()?;

                    tracing::debug!(
                        "Applying migration `{}` to shadow database.",
                        migration.migration_name()
                    );

                    temp_database
                        .raw_cmd(&script)
                        .await
                        .map_err(ConnectorError::from)
                        .map_err(|connector_error| {
                            connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                        })?;
                }

                self.describe_schema(&temp_database).await
            })()
            .await
        };

        if let Some(shadow_database_name) = shadow_database_name {
            self.clean_up_shadow_database(connection, &shadow_database_name).await?;
        }

        sql_schema_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_impl_does_not_leak_connection_info() {
        let url = "sqlserver://myserver:8765;database=master;schema=mydbname;user=SA;password=<mypassword>;trustServerCertificate=true;socket_timeout=60;isolationLevel=READ UNCOMMITTED";

        let flavour = MssqlFlavour::new(MssqlUrl::new(url).unwrap(), BitFlags::empty());
        let debugged = format!("{:?}", flavour);

        let words = &["myname", "mypassword", "myserver", "8765", "mydbname"];

        for word in words {
            assert!(!debugged.contains(word));
        }
    }
}
