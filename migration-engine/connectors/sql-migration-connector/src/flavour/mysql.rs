use super::SqlFlavour;
use crate::{
    connect,
    connection_wrapper::Connection,
    error::{quaint_error_to_connector_error, SystemDatabase},
};
use datamodel::{walkers::walk_scalar_fields, Datamodel};
use enumflags2::BitFlags;
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory};
use once_cell::sync::Lazy;
use quaint::{connector::MysqlUrl, prelude::SqlFamily};
use regex::RegexSet;
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use std::sync::atomic::{AtomicU8, Ordering};
use url::Url;

#[derive(Debug)]
pub(crate) struct MysqlFlavour {
    pub(super) url: MysqlUrl,
    /// See the [Circumstances] enum.
    pub(super) circumstances: AtomicU8,
}

impl MysqlFlavour {
    pub(crate) fn is_mariadb(&self) -> bool {
        BitFlags::<Circumstances>::from_bits(self.circumstances.load(Ordering::Relaxed))
            .unwrap_or_default()
            .contains(Circumstances::IsMariadb)
    }

    pub(crate) fn is_mysql_5_6(&self) -> bool {
        BitFlags::<Circumstances>::from_bits(self.circumstances.load(Ordering::Relaxed))
            .unwrap_or_default()
            .contains(Circumstances::IsMysql56)
    }

    pub(crate) fn lower_cases_table_names(&self) -> bool {
        BitFlags::<Circumstances>::from_bits(self.circumstances.load(Ordering::Relaxed))
            .unwrap_or_default()
            .contains(Circumstances::LowerCasesTableNames)
    }
}

#[async_trait::async_trait]
impl SqlFlavour for MysqlFlavour {
    fn check_database_version_compatibility(
        &self,
        datamodel: &Datamodel,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        if self.is_mysql_5_6() {
            let mut errors = Vec::new();

            check_datamodel_for_mysql_5_6(datamodel, &mut errors);

            if errors.is_empty() {
                return None;
            }

            let mut errors_string = String::with_capacity(errors.iter().map(|err| err.len() + 3).sum());

            for error in &errors {
                errors_string.push_str("- ");
                errors_string.push_str(error);
                errors_string.push_str("\n");
            }

            Some(user_facing_errors::common::DatabaseVersionIncompatibility {
                errors: errors_string,
                database_version: "MySQL 5.6".into(),
            })
        } else {
            None
        }
    }

    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        url.set_path("/mysql");

        let conn = connect(&url.to_string()).await?;
        let db_name = self.url.dbname();

        let query = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
            db_name
        );

        conn.raw_cmd(&query).await?;

        Ok(db_name.to_owned())
    }

    async fn create_imperative_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE _prisma_migrations (
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

        Ok(connection.raw_cmd(sql).await?)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::mysql::SqlSchemaDescriber::new(connection.quaint().clone())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    quaint_error_to_connector_error(err, connection.connection_info())
                }
            })
    }

    async fn drop_database(&self, database_url: &str) -> ConnectorResult<()> {
        let connection = connect(database_url).await?;
        let db_name = connection.connection_info().dbname().unwrap();

        connection.raw_cmd(&format!("DROP DATABASE `{}`", db_name)).await?;

        Ok(())
    }

    async fn ensure_connection_validity(&self, connection: &Connection) -> ConnectorResult<()> {
        static MYSQL_SYSTEM_DATABASES: Lazy<regex::RegexSet> = Lazy::new(|| {
            RegexSet::new(&[
                "(?i)^mysql$",
                "(?i)^information_schema$",
                "(?i)^performance_schema$",
                "(?i)^sys$",
            ])
            .unwrap()
        });

        let db_name = connection.connection_info().schema_name();

        if MYSQL_SYSTEM_DATABASES.is_match(db_name) {
            return Err(SystemDatabase(db_name.to_owned()).into());
        }

        let version = connection.version().await?;
        let mut circumstances = BitFlags::<Circumstances>::default();

        if let Some(version) = version {
            if version.starts_with("5.6") {
                circumstances |= Circumstances::IsMysql56;
            }

            if version.contains("MariaDB") {
                circumstances |= Circumstances::IsMariadb;
            }
        }

        let result_set = connection
            .query_raw("SHOW VARIABLES WHERE variable_name = 'lower_case_table_names'", &[])
            .await?;

        if let Some((setting_name, setting_value)) = result_set.into_single().ok().and_then(|row| {
            let setting_name = row.at(0).and_then(|row| row.to_string())?;
            let setting_value = row.at(1).and_then(|row| row.as_i64())?;
            Some((setting_name, setting_value))
        }) {
            assert_eq!(setting_name, "lower_case_table_names");

            // https://dev.mysql.com/doc/refman/8.0/en/identifier-case-sensitivity.html
            if setting_value == 2 {
                circumstances |= Circumstances::LowerCasesTableNames;
            }
        }

        self.circumstances.store(circumstances.bits(), Ordering::Relaxed);

        Ok(())
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        url.set_path("/mysql");

        let conn = connect(&url.to_string()).await?;
        let db_name = self.url.dbname();

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

        let mut temporary_database_url = self.url.url().clone();
        temporary_database_url.set_path(&format!("/{}", database_name));
        let temporary_database_url = temporary_database_url.to_string();

        tracing::debug!("Connecting to temporary database at {:?}", temporary_database_url);

        let temp_database = crate::connect(&temporary_database_url).await?;

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

        let sql_schema = self.describe_schema(&temp_database).await?;

        connection.raw_cmd(&drop_database).await?;

        Ok(sql_schema)
    }
}

#[derive(BitFlags, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Circumstances {
    LowerCasesTableNames = 0b0001,
    IsMysql56 = 0b0010,
    IsMariadb = 0b0100,
}

fn check_datamodel_for_mysql_5_6(datamodel: &Datamodel, errors: &mut Vec<String>) {
    walk_scalar_fields(datamodel).for_each(|field| {
        if field.field_type().is_json() {
            errors.push(format!(
                "The `Json` data type used in {}.{} is not supported on MySQL 5.6.",
                field.model().name(),
                field.name()
            ))
        }
    });
}
