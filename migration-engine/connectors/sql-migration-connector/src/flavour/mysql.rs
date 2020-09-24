use super::SqlFlavour;
use crate::{
    catch, connect, database_info::DatabaseInfo, error::CheckDatabaseInfoResult, error::SystemDatabase, SqlError,
    SqlResult,
};
use futures::TryFutureExt;
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory};
use once_cell::sync::Lazy;
use quaint::{connector::MysqlUrl, prelude::ConnectionInfo, prelude::Queryable, prelude::SqlFamily, single::Quaint};
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

        let (conn, _) = connect(&url.to_string()).await?;
        let db_name = self.0.dbname();

        let query = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
            db_name
        );
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        Ok(db_name.to_owned())
    }

    async fn describe_schema<'a>(&'a self, schema_name: &'a str, conn: Quaint) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::mysql::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
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

        catch(connection_info, connection.raw_cmd(sql).map_err(SqlError::from)).await
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        url.set_path("/mysql");

        let (conn, _) = connect(&url.to_string()).await?;

        let db_name = self.0.dbname();

        let query = format!("DROP DATABASE IF EXISTS `{}`", db_name);
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        let query = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
            db_name
        );
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        Ok(())
    }

    async fn reset(&self, connection: &dyn Queryable, connection_info: &ConnectionInfo) -> ConnectorResult<()> {
        let db_name = connection_info.dbname().unwrap();

        catch(
            connection_info,
            connection
                .raw_cmd(&format!("DROP DATABASE `{}`", db_name))
                .map_err(SqlError::from),
        )
        .await?;

        catch(
            connection_info,
            connection
                .raw_cmd(&format!("CREATE DATABASE `{}`", db_name))
                .map_err(SqlError::from),
        )
        .await?;

        catch(
            connection_info,
            connection
                .raw_cmd(&format!("USE `{}`", db_name))
                .map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mysql
    }

    #[tracing::instrument(skip(self, migrations, connection, connection_info))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<SqlSchema> {
        let database_name = format!("prisma_shadow_db{}", uuid::Uuid::new_v4());
        let drop_database = format!("DROP DATABASE IF EXISTS `{}`", database_name);
        let create_database = format!("CREATE DATABASE `{}`", database_name);

        catch(
            connection_info,
            connection.raw_cmd(&drop_database).map_err(SqlError::from),
        )
        .await?;
        catch(
            connection_info,
            connection.raw_cmd(&create_database).map_err(SqlError::from),
        )
        .await?;

        let mut temporary_database_url = self.0.url().clone();
        temporary_database_url.set_path(&format!("/{}", database_name));
        let temporary_database_url = temporary_database_url.to_string();

        tracing::debug!("Connecting to temporary database at {:?}", temporary_database_url);

        let quaint = catch(
            connection_info,
            Quaint::new(&temporary_database_url).map_err(SqlError::from),
        )
        .await?;

        for migration in migrations {
            let script = migration
                .read_migration_script()
                .expect("failed to read migration script");

            tracing::debug!(
                "Applying migration `{}` to temporary database.",
                migration.migration_name()
            );

            catch(
                quaint.connection_info(),
                quaint.raw_cmd(&script).map_err(SqlError::from),
            )
            .await
            .map_err(|connector_error| connector_error.into_migration_failed(migration.migration_name().to_owned()))?;
        }

        let connection_info = quaint.connection_info().clone();

        let sql_schema = catch(
            &connection_info,
            self.describe_schema(connection_info.schema_name(), quaint),
        )
        .await?;

        catch(
            &connection_info,
            connection.raw_cmd(&drop_database).map_err(SqlError::from),
        )
        .await?;

        Ok(sql_schema)
    }
}
