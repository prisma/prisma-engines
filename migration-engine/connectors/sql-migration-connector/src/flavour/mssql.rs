use super::SqlFlavour;
use crate::{catch, connect, SqlError, SqlResult};
use futures::TryFutureExt;
use migration_connector::{ConnectorResult, MigrationDirectory};
use quaint::{connector::MssqlUrl, prelude::ConnectionInfo, prelude::Queryable, prelude::SqlFamily, single::Quaint};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
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
        let (conn, _) = connect(&master_uri.to_string()).await?;

        let query = format!("CREATE DATABASE [{}]", db_name);
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        let (conn, _) = connect(jdbc_string).await?;

        let query = format!("CREATE SCHEMA {}", conn.connection_info().schema_name());
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        Ok(db_name)
    }

    async fn describe_schema<'a>(&'a self, schema_name: &'a str, conn: Quaint) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::mssql::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn reset(&self, connection: &dyn Queryable, connection_info: &ConnectionInfo) -> ConnectorResult<()> {
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
            connection_info.schema_name()
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
            connection_info.schema_name()
        );

        catch(connection_info, connection.raw_cmd(&drop_fks).map_err(SqlError::from)).await?;

        catch(
            connection_info,
            connection.raw_cmd(&drop_tables).map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let (db_name, master_uri) = Self::master_url(database_str);
        let (conn, info) = connect(&master_uri).await?;

        // Without these, our poor connection gets deadlocks if other schemas
        // are modified while we introspect.
        let allow_snapshot_isolation = format!(
            "ALTER DATABASE [{db_name}] SET ALLOW_SNAPSHOT_ISOLATION ON",
            db_name = db_name
        );

        conn.raw_cmd(&allow_snapshot_isolation).await.unwrap();

        self.reset(&conn, info.connection_info()).await?;

        conn.raw_cmd(&format!(
            "DROP SCHEMA IF EXISTS {}",
            info.connection_info().schema_name()
        ))
        .await
        .unwrap();

        conn.raw_cmd(&format!("CREATE SCHEMA {}", info.connection_info().schema_name()))
            .await
            .unwrap();

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mssql
    }

    async fn ensure_connection_validity(&self, connection: &Quaint) -> ConnectorResult<()> {
        catch(
            connection.connection_info(),
            connection.raw_cmd("SELECT 1").map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    async fn ensure_imperative_migrations_table(
        &self,
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<()> {
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

        catch(connection_info, connection.raw_cmd(sql).map_err(SqlError::from)).await
    }

    async fn sql_schema_from_migration_history(
        &self,
        _: &[MigrationDirectory],
        _: &dyn Queryable,
        _: &ConnectionInfo,
    ) -> ConnectorResult<SqlSchema> {
        todo!("Needs the connection string crate, so leaving it unimplemented for now")
    }
}
