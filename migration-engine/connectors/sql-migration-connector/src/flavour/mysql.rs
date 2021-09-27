use super::SqlFlavour;
use crate::{
    connection_wrapper::{connect, quaint_error_to_connector_error, Connection},
    error::SystemDatabase,
    SqlMigrationConnector,
};
use datamodel::{common::preview_features::PreviewFeature, walkers::walk_scalar_fields, Datamodel};
use enumflags2::BitFlags;
use indoc::indoc;
use migration_connector::{migrations_directory::MigrationDirectory, ConnectorError, ConnectorResult};
use once_cell::sync::Lazy;
use quaint::connector::{
    mysql_async::{self as my, prelude::Query},
    MysqlUrl,
};
use regex::{Regex, RegexSet};
use sql_schema_describer::SqlSchema;
use std::sync::atomic::{AtomicU8, Ordering};
use url::Url;
use user_facing_errors::{
    migration_engine::{ApplyMigrationError, DirectDdlNotAllowed, ForeignKeyCreationNotAllowed},
    KnownError,
};

const ADVISORY_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
static QUALIFIED_NAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"`[^ ]+`\.`[^ ]+`"#).unwrap());

pub(crate) struct MysqlFlavour {
    url: MysqlUrl,
    /// See the [Circumstances] enum.
    circumstances: AtomicU8,
    preview_features: BitFlags<PreviewFeature>,
}

impl std::fmt::Debug for MysqlFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MysqlFlavour").field("url", &"<REDACTED>").finish()
    }
}

impl MysqlFlavour {
    pub(crate) fn new(url: MysqlUrl, preview_features: BitFlags<PreviewFeature>) -> Self {
        MysqlFlavour {
            url,
            circumstances: Default::default(),
            preview_features,
        }
    }

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

    pub(crate) fn is_vitess(&self) -> bool {
        BitFlags::<Circumstances>::from_bits(self.circumstances.load(Ordering::Relaxed))
            .unwrap_or_default()
            .contains(Circumstances::IsVitess)
    }

    pub(crate) fn lower_cases_table_names(&self) -> bool {
        BitFlags::<Circumstances>::from_bits(self.circumstances.load(Ordering::Relaxed))
            .unwrap_or_default()
            .contains(Circumstances::LowerCasesTableNames)
    }

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

            super::validate_connection_infos_do_not_match((shadow_conninfo, main_conninfo))?;

            tracing::info!(
                "Connecting to user-provided shadow database at {}.{:?}",
                shadow_conninfo.host(),
                shadow_conninfo.dbname()
            );

            if self.reset(&conn).await.is_err() {
                connector.best_effort_reset(&conn).await?;
            }

            return Ok(conn);
        }

        let database_name = shadow_database_name.unwrap();
        let create_database = format!("CREATE DATABASE `{}`", database_name);

        main_connection
            .raw_cmd(&create_database)
            .await
            .map_err(ConnectorError::from)
            .map_err(|err| err.into_shadow_db_creation_error())?;

        let mut shadow_database_url = self.url.url().clone();
        shadow_database_url.set_path(&format!("/{}", database_name));
        let host = shadow_database_url.host();
        let shadow_database_url = shadow_database_url.to_string();

        tracing::debug!("Connecting to shadow database at {:?}/{}", host, database_name);

        Ok(crate::connect(&shadow_database_url).await?)
    }

    fn convert_server_error(&self, error: &my::Error) -> Option<KnownError> {
        if self.is_vitess() {
            match error {
                my::Error::Server(se) if se.code == 1317 => Some(KnownError::new(ForeignKeyCreationNotAllowed)),
                // sigh, this code is for unknown error, so we have the ddl
                // error and other stuff, such as typos in the same category...
                my::Error::Server(se) if se.code == 1105 && se.message == "direct DDL is disabled" => {
                    Some(KnownError::new(DirectDdlNotAllowed))
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

#[async_trait::async_trait]
impl SqlFlavour for MysqlFlavour {
    async fn acquire_lock(&self, connection: &Connection) -> ConnectorResult<()> {
        // https://dev.mysql.com/doc/refman/8.0/en/locking-functions.html
        let query = format!("SELECT GET_LOCK('prisma_migrate', {})", ADVISORY_LOCK_TIMEOUT.as_secs());
        Ok(connection.raw_cmd(&query).await?)
    }

    async fn run_query_script(&self, sql: &str, connection: &Connection) -> ConnectorResult<()> {
        let convert_error = |error: my::Error| match self.convert_server_error(&error) {
            Some(e) => ConnectorError::from(e),
            None => quaint_error_to_connector_error(quaint::error::Error::from(error), connection.connection_info()),
        };

        let (client, _url) = connection.unwrap_mysql();
        let mut conn = client.conn().lock().await;

        let mut result = sql.run(&mut *conn).await.map_err(convert_error)?;

        loop {
            match result.map(drop).await {
                Ok(_) => {
                    if result.is_empty() {
                        result.map(drop).await.map_err(convert_error)?;
                        return Ok(());
                    }
                }
                Err(e) => {
                    return Err(convert_error(e));
                }
            }
        }
    }

    async fn apply_migration_script(
        &self,
        migration_name: &str,
        script: &str,
        connection: &Connection,
    ) -> ConnectorResult<()> {
        let convert_error = |migration_idx: usize, error: my::Error| {
            let position = format!(
                "Please check the query number {} from the migration file.",
                migration_idx + 1
            );

            let (code, error) = match (&error, self.convert_server_error(&error)) {
                (my::Error::Server(se), Some(error)) => {
                    let message = format!("{}\n\n{}", error.message, position);
                    (Some(se.code), message)
                }
                (my::Error::Server(se), None) => {
                    let message = format!("{}\n\n{}", se.message, position);
                    (Some(se.code), message)
                }
                _ => (None, error.to_string()),
            };

            ConnectorError::user_facing(ApplyMigrationError {
                migration_name: migration_name.to_owned(),
                database_error_code: code.map(|c| c.to_string()).unwrap_or_else(|| String::from("none")),
                database_error: error,
            })
        };

        let (client, _url) = connection.unwrap_mysql();
        let mut conn = client.conn().lock().await;

        let mut migration_idx = 0_usize;

        let mut result = script
            .run(&mut *conn)
            .await
            .map_err(|e| convert_error(migration_idx, e))?;

        loop {
            match result.map(drop).await {
                Ok(_) => {
                    migration_idx += 1;

                    if result.is_empty() {
                        result.map(drop).await.map_err(|e| convert_error(migration_idx, e))?;
                        return Ok(());
                    }
                }
                Err(e) => {
                    return Err(convert_error(migration_idx, e));
                }
            }
        }
    }

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
                errors_string.push('\n');
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
        let mut url = Url::parse(database_str).map_err(ConnectorError::url_parse_error)?;
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

    async fn create_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = indoc! {r#"
            CREATE TABLE _prisma_migrations (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             DATETIME(3),
                migration_name          VARCHAR(255) NOT NULL,
                logs                    TEXT,
                rolled_back_at          DATETIME(3),
                started_at              DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
                applied_steps_count     INTEGER UNSIGNED NOT NULL DEFAULT 0
            ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
        "#};

        Ok(self.run_query_script(sql, connection).await?)
    }

    async fn drop_database(&self, database_url: &str) -> ConnectorResult<()> {
        let connection = connect(database_url).await?;
        let connection_info = connection.connection_info();
        let db_name = connection_info.dbname().unwrap();

        connection.raw_cmd(&format!("DROP DATABASE `{}`", db_name)).await?;

        Ok(())
    }

    async fn drop_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        connection.raw_cmd("DROP TABLE _prisma_migrations").await?;

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

        let version = connection
            .query_raw("SELECT @@version", &[])
            .await?
            .into_iter()
            .next()
            .and_then(|r| r.into_iter().next())
            .and_then(|val| val.into_string());

        let global_version = connection.version().await?;
        let mut circumstances = BitFlags::<Circumstances>::default();

        if let Some(version) = version {
            if version.contains("vitess") || version.contains("Vitess") {
                circumstances |= Circumstances::IsVitess;
            }
        }

        if let Some(version) = global_version {
            if version.starts_with("5.6") {
                circumstances |= Circumstances::IsMysql56;
            }

            if version.contains("MariaDB") {
                circumstances |= Circumstances::IsMariadb;
            }
        }

        let result_set = connection.query_raw("SELECT @@lower_case_table_names", &[]).await?;

        if let Some(1) = result_set.into_single().ok().and_then(|row| {
            row.at(0)
                .and_then(|row| row.to_string().and_then(|s| s.parse().ok()).or_else(|| row.as_i64()))
        }) {
            // https://dev.mysql.com/doc/refman/8.0/en/identifier-case-sensitivity.html
            circumstances |= Circumstances::LowerCasesTableNames;
        }

        self.circumstances.store(circumstances.bits(), Ordering::Relaxed);

        Ok(())
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(ConnectorError::url_parse_error)?;
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
        if self.is_vitess() {
            return Err(ConnectorError::from_msg(
                "We do not drop databases on Vitess until it works better.".into(),
            ));
        }

        let connection_info = connection.connection_info();
        let db_name = connection_info.dbname().unwrap();

        connection.raw_cmd(&format!("DROP DATABASE `{}`", db_name)).await?;
        connection.raw_cmd(&format!("CREATE DATABASE `{}`", db_name)).await?;
        connection.raw_cmd(&format!("USE `{}`", db_name)).await?;

        Ok(())
    }

    fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.preview_features
    }

    fn scan_migration_script(&self, script: &str) {
        for capture in QUALIFIED_NAME_RE
            .captures_iter(script)
            .filter_map(|captures| captures.get(0))
        {
            tracing::warn!(
                location = ?capture.range(),
                name = capture.as_str(),
                "Your migration appears to contain a qualified name. Qualified names like `mydb`.`mytable` interact badly with the shadow database on MySQL. Please change these to unqualified names (just `mytable` in the previous example)."
            );
        }
    }

    #[tracing::instrument(skip(self, migrations, connection, connector))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
        connector: &SqlMigrationConnector,
    ) -> ConnectorResult<SqlSchema> {
        let shadow_database_name = connector.shadow_database_name();

        let temp_database = self
            .shadow_database_connection(connection, connector, shadow_database_name.as_deref())
            .await?;

        // We go through the whole process without early return, then clean up
        // the shadow database, and only then return the result. This avoids
        // leaving shadow databases behind in case of e.g. faulty migrations.

        let sql_schema_result = (|| async {
            for migration in migrations {
                let script = migration.read_migration_script()?;

                tracing::debug!(
                    "Applying migration `{}` to shadow database.",
                    migration.migration_name()
                );

                self.scan_migration_script(&script);

                self.apply_migration_script(migration.migration_name(), &script, &temp_database)
                    .await
                    .map_err(|connector_error| {
                        connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                    })?;
            }

            temp_database.describe_schema().await
        })()
        .await;

        if let Some(database_name) = shadow_database_name {
            let drop_database = format!("DROP DATABASE IF EXISTS `{}`", database_name);
            connection.raw_cmd(&drop_database).await?;
        }

        sql_schema_result
    }
}

#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum Circumstances {
    LowerCasesTableNames,
    IsMysql56,
    IsMariadb,
    IsVitess,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_impl_does_not_leak_connection_info() {
        let url = "mysql://myname:mypassword@myserver:8765/mydbname";

        let flavour = MysqlFlavour::new(MysqlUrl::new(url.parse().unwrap()).unwrap(), BitFlags::empty()); // unwrap this
        let debugged = format!("{:?}", flavour);

        let words = &["myname", "mypassword", "myserver", "8765", "mydbname"];

        for word in words {
            assert!(!debugged.contains(word));
        }
    }

    #[test]
    fn qualified_name_re_matches_as_expected() {
        let should_match = r#"ALTER TABLE `mydb`.`cat` DROP PRIMARY KEY"#;
        let should_not_match = r#"ALTER TABLE `cat` ADD FOREIGN KEY (`ab`, cd`) REFERENCES `dog`(`id`)"#;

        assert!(
            QUALIFIED_NAME_RE.is_match_at(should_match, 12),
            "captures: {:?}",
            QUALIFIED_NAME_RE.captures(should_match)
        );
        assert!(!QUALIFIED_NAME_RE.is_match(should_not_match));
    }
}
