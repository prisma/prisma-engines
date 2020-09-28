use super::SqlFlavour;
use crate::{connect, connection_wrapper::Connection};
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory};
use quaint::{connector::MssqlUrl, prelude::SqlFamily};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend, SqlSchemaDescriberError};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct MssqlFlavour(pub(crate) MssqlUrl);

impl MssqlFlavour {
    pub(crate) fn schema_name(&self) -> &str {
        self.0.schema()
    }

    fn master_url(jdbc_string: &str) -> (String, String) {
        let mut splitted = jdbc_string.split(';');
        let uri = splitted.next().unwrap().to_string();

        let mut params: HashMap<String, String> = splitted
            .map(|kv| kv.split('='))
            .map(|mut kv| {
                let key = kv.next().unwrap().to_string();
                let value = kv.next().unwrap().to_string();

                (key, value)
            })
            .collect();

        let db_name = params.remove("database").unwrap_or_else(|| String::from("master"));
        let params: Vec<_> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();

        let master_uri = format!("{};{}", uri, params.join(";"));

        (db_name, master_uri)
    }
}

#[async_trait::async_trait]
impl SqlFlavour for MssqlFlavour {
    async fn create_database(&self, jdbc_string: &str) -> ConnectorResult<String> {
        let (db_name, master_uri) = Self::master_url(jdbc_string);
        let conn = connect(&master_uri.to_string()).await?;

        let query = format!("CREATE DATABASE [{}]", db_name);
        conn.raw_cmd(&query).await?;

        let conn = connect(jdbc_string).await?;

        let query = format!("CREATE SCHEMA {}", conn.connection_info().schema_name());
        conn.raw_cmd(&query).await?;

        Ok(db_name)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::mssql::SqlSchemaDescriber::new(connection.quaint().clone())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err {
                SqlSchemaDescriberError::UnknownError => {
                    ConnectorError::query_error(anyhow::anyhow!("An unknown error occurred in sql-schema-describer"))
                }
            })
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
        let (db_name, master_uri) = Self::master_url(database_str);
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

        conn.raw_cmd(&format!("CREATE SCHEMA {}", conn.connection_info().schema_name()))
            .await
            .unwrap();

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mssql
    }

    async fn ensure_connection_validity(&self, connection: &Connection) -> ConnectorResult<()> {
        connection.raw_cmd("SELECT 1").await?;

        Ok(())
    }

    async fn ensure_imperative_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS [_prisma_migrations] (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             DATETIMEOFFSET,
                migration_name          NVARCHAR(MAX) NOT NULL,
                logs                    NVARCHAR(MAX) NOT NULL,
                rolled_back_at          DATETIMEOFFSET,
                started_at              DATETIMEOFFSET NOT NULL DEFAULT CURRENT_TIMESTAMP,
                applied_steps_count     INT NOT NULL DEFAULT 0,
                script                  NVARCHAR(MAX) NOT NULL
            );
        "#;

        connection.raw_cmd(sql).await
    }

    async fn sql_schema_from_migration_history(
        &self,
        _: &[MigrationDirectory],
        _: &Connection,
    ) -> ConnectorResult<SqlSchema> {
        todo!("Needs the connection string crate, so leaving it unimplemented for now")
    }
}
