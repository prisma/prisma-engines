use crate::{connect, connection_wrapper::Connection, error::quaint_error_to_connector_error, SqlFlavour};
use connection_string::JdbcString;
use indoc::formatdoc;
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory};
use quaint::{
    connector::MssqlUrl,
    prelude::{SqlFamily, Table},
};
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use std::str::FromStr;

#[derive(Debug)]
pub(crate) struct MssqlFlavour(pub(crate) MssqlUrl);

impl MssqlFlavour {
    pub(crate) fn schema_name(&self) -> &str {
        self.0.schema()
    }

    /// Get the url as a JDBC string, extract the database name, and re-encode the string.
    fn master_url(input: &str) -> ConnectorResult<(String, String)> {
        let mut conn = JdbcString::from_str(&format!("jdbc:{}", input))
            .map_err(|e| ConnectorError::generic(anyhow::Error::new(e)))?;
        let params = conn.properties_mut();

        let db_name = params.remove("database").unwrap_or_else(|| String::from("master"));
        Ok((db_name, conn.to_string()))
    }
}

#[async_trait::async_trait]
impl SqlFlavour for MssqlFlavour {
    fn imperative_migrations_table(&self) -> Table<'_> {
        (self.schema_name(), self.imperative_migrations_table_name()).into()
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

    async fn create_imperative_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
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
        "#, self.schema_name(), self.imperative_migrations_table_name()};

        Ok(connection.raw_cmd(&sql).await?)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::mssql::SqlSchemaDescriber::new(connection.quaint().clone())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    quaint_error_to_connector_error(err, connection.connection_info())
                }
            })
    }

    async fn drop_database(&self, _database_url: &str) -> ConnectorResult<()> {
        let features = vec!["microsoftSqlServer".into()];
        return Err(ConnectorError::user_facing_error(
            user_facing_errors::migration_engine::PreviewFeaturesBlocked { features },
        ));
    }

    async fn reset(&self, connection: &Connection) -> ConnectorResult<()> {
        let schema_name = connection.connection_info().schema_name();
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

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mssql
    }

    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
    ) -> ConnectorResult<SqlSchema> {
        let database_name = format!("prisma_shadow_db{}", uuid::Uuid::new_v4());
        let create_database = format!("CREATE DATABASE [{}]", database_name);

        connection.raw_cmd(&create_database).await?;

        let mut jdbc_string: JdbcString = self.0.connection_string().parse().unwrap();
        jdbc_string
            .properties_mut()
            .insert("database".into(), database_name.clone());
        let temporary_database_url = jdbc_string.to_string();

        tracing::debug!("Connecting to temporary database at `{}`", temporary_database_url);

        // We must create a connection in a block, closing it before dropping
        // the database.
        let sql_schema_result = {
            let temp_database = crate::connect(&temporary_database_url).await?;

            // We go through the whole process without early return, then clean up
            // the temporary database, and only then return the result. This avoids
            // leaving shadow databases behind in case of e.g. faulty
            // migrations.

            if self.schema_name() != "dbo" {
                let create_schema = format!("CREATE SCHEMA [{schema}]", schema = self.schema_name());

                temp_database.raw_cmd(&create_schema).await?;
            }

            (|| async {
                for migration in migrations {
                    let script = migration.read_migration_script()?;

                    tracing::debug!(
                        "Applying migration `{}` to temporary database.",
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

        let drop_database = format!("DROP DATABASE [{}]", database = database_name);
        connection.raw_cmd(&drop_database).await?;

        sql_schema_result
    }
}
