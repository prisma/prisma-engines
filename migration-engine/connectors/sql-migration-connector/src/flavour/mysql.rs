mod shadow_db;

use crate::{
    connection_wrapper::{connect, quaint_error_to_connector_error, Connection},
    error::SystemDatabase,
    flavour::{normalize_sql_schema, SqlFlavour},
};
use datamodel::{parser_database::ScalarType, ValidatedSchema};
use enumflags2::BitFlags;
use indoc::indoc;
use migration_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult,
};
use once_cell::sync::Lazy;
use quaint::connector::{
    mysql_async::{self as my, prelude::Query},
    MysqlUrl,
};
use regex::{Regex, RegexSet};
use sql_schema_describer::SqlSchema;
use std::future;
use url::Url;
use user_facing_errors::{
    migration_engine::{ApplyMigrationError, DirectDdlNotAllowed, ForeignKeyCreationNotAllowed},
    KnownError,
};

const ADVISORY_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
static QUALIFIED_NAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"`[^ ]+`\.`[^ ]+`"#).unwrap());

type State = super::State<Params, (BitFlags<Circumstances>, Connection)>;

struct Params {
    connector_params: ConnectorParams,
    url: MysqlUrl,
}

pub(crate) struct MysqlFlavour {
    state: State,
}

impl Default for MysqlFlavour {
    fn default() -> Self {
        MysqlFlavour { state: State::Initial }
    }
}

impl std::fmt::Debug for MysqlFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MysqlFlavour").finish()
    }
}

impl MysqlFlavour {
    pub(crate) fn is_mariadb(&self) -> bool {
        self.circumstances().contains(Circumstances::IsMariadb)
    }

    pub(crate) fn is_mysql_5_6(&self) -> bool {
        self.circumstances().contains(Circumstances::IsMysql56)
    }

    pub(crate) fn lower_cases_table_names(&self) -> bool {
        self.circumstances().contains(Circumstances::LowerCasesTableNames)
    }

    fn circumstances(&self) -> BitFlags<Circumstances> {
        match self.state {
            super::State::Initial | super::State::WithParams(_) => Default::default(),
            super::State::Connected(_, (c, _)) => c,
        }
    }
}

impl SqlFlavour for MysqlFlavour {
    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, |_, _, connection| async move {
            // https://dev.mysql.com/doc/refman/8.0/en/locking-functions.html
            let query = format!("SELECT GET_LOCK('prisma_migrate', {})", ADVISORY_LOCK_TIMEOUT.as_secs());
            Ok(connection.raw_cmd(&query).await?)
        })
    }

    fn connector_type(&self) -> &'static str {
        "mysql"
    }

    fn datamodel_connector(&self) -> &'static dyn datamodel::datamodel_connector::Connector {
        sql_datamodel_connector::MYSQL
    }

    fn describe_schema(&mut self) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        use sql_schema_describer::{mysql as describer, DescriberErrorKind, SqlSchemaDescriberBackend};
        with_connection(&mut self.state, |params, _circumstances, connection| async move {
            let mut schema = describer::SqlSchemaDescriber::new(connection.queryable())
                .describe(params.url.dbname())
                .await
                .map_err(|err| match err.into_kind() {
                    DescriberErrorKind::QuaintError(err) => quaint_error_to_connector_error(
                        err,
                        &quaint::prelude::ConnectionInfo::Mysql(params.url.clone()),
                    ),
                    DescriberErrorKind::CrossSchemaReference { .. } => {
                        unreachable!("No schemas on MySQL")
                    }
                })?;

            normalize_sql_schema(&mut schema, params.connector_params.preview_features);
            Ok(schema)
        })
    }

    fn run_query_script<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |_params, circumstances, connection| async move {
            let convert_error = |error: my::Error| match convert_server_error(circumstances, &error) {
                Some(e) => ConnectorError::from(e),
                None => {
                    quaint_error_to_connector_error(quaint::error::Error::from(error), connection.connection_info())
                }
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
        })
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |_params, circumstances, connection| async move {
            let convert_error = |migration_idx: usize, error: my::Error| {
                let position = format!(
                    "Please check the query number {} from the migration file.",
                    migration_idx + 1
                );

                let (code, error) = match (&error, convert_server_error(circumstances, &error)) {
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
        })
    }

    fn check_database_version_compatibility(
        &self,
        datamodel: &ValidatedSchema,
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

    fn connection_string(&self) -> Option<&str> {
        self.state
            .params()
            .map(|p| p.connector_params.connection_string.as_str())
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            let params = self.state.get_unwrapped_params();
            let mut url =
                Url::parse(&params.connector_params.connection_string).map_err(ConnectorError::url_parse_error)?;
            url.set_path("/mysql");

            let conn = connect(&url.to_string()).await?;
            let db_name = params.url.dbname();

            let query = format!(
                "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
                db_name
            );

            conn.raw_cmd(&query).await?;

            Ok(db_name.to_owned())
        })
    }

    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
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

        self.run_query_script(sql)
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async {
            let params = self.state.get_unwrapped_params();
            let connection = connect(&params.connector_params.connection_string).await?;
            let connection_info = connection.connection_info();
            let db_name = connection_info.dbname().unwrap();

            connection.raw_cmd(&format!("DROP DATABASE `{}`", db_name)).await?;

            Ok(())
        })
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("DROP TABLE _prisma_migrations")
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, |_, _, _| future::ready(Ok(())))
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(
            &mut self.state,
            |_, _, conn| async move { Ok(conn.query(query).await?) },
        )
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(&mut self.state, move |_, _, conn| async move {
            Ok(conn.query_raw(sql, params).await?)
        })
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |_, _, conn| async move {
            Ok(conn.raw_cmd(sql).await?)
        })
    }

    fn reset(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, move |params, circumstances, connection| async move {
            if circumstances.contains(Circumstances::IsVitess) {
                return Err(ConnectorError::from_msg(
                    "We do not drop databases on Vitess until it works better.".into(),
                ));
            }

            let db_name = params.url.dbname();
            connection.raw_cmd(&format!("DROP DATABASE `{}`", db_name)).await?;
            connection.raw_cmd(&format!("CREATE DATABASE `{}`", db_name)).await?;
            connection.raw_cmd(&format!("USE `{}`", db_name)).await?;

            Ok(())
        })
    }

    fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()> {
        let url: Url = params
            .connection_string
            .parse()
            .map_err(ConnectorError::url_parse_error)?;
        let url = quaint::connector::MysqlUrl::new(url).map_err(ConnectorError::url_parse_error)?;
        let params = Params {
            connector_params: params,
            url,
        };
        self.state.set_params(params);
        Ok(())
    }

    fn scan_migration_script(&self, script: &str) {
        scan_migration_script_impl(script)
    }

    #[tracing::instrument(skip(self, migrations))]
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        shadow_database_connection_string: Option<String>,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        let shadow_database_connection_string = shadow_database_connection_string.or_else(|| {
            self.state
                .params()
                .and_then(|p| p.connector_params.shadow_database_connection_string.clone())
        });
        let mut shadow_database = MysqlFlavour::default();

        match shadow_database_connection_string {
            Some(shadow_database_connection_string) => Box::pin(async move {
                if let Some(params) = self.state.params() {
                    super::validate_connection_infos_do_not_match(
                        &shadow_database_connection_string,
                        &params.connector_params.connection_string,
                    )?;
                }

                let shadow_db_params = ConnectorParams {
                    connection_string: shadow_database_connection_string.to_owned(),
                    preview_features: self
                        .state
                        .params()
                        .map(|p| p.connector_params.preview_features)
                        .unwrap_or_default(),
                    shadow_database_connection_string: None,
                };

                shadow_database.set_params(shadow_db_params)?;
                shadow_database.ensure_connection_validity().await?;

                tracing::info!("Connecting to user-provided shadow database.");
                if shadow_database.reset().await.is_err() {
                    crate::best_effort_reset(&mut shadow_database).await?;
                }

                shadow_db::sql_schema_from_migrations_history(migrations, shadow_database).await
            }),
            None => {
                with_connection(&mut self.state, move |params, _circumstances, conn| async move {
                    let shadow_database_name = crate::new_shadow_database_name();

                    let create_database = format!("CREATE DATABASE `{}`", shadow_database_name);
                    conn.raw_cmd(&create_database)
                        .await
                        .map_err(ConnectorError::from)
                        .map_err(|err| err.into_shadow_db_creation_error())?;

                    let mut shadow_database_url = params.url.url().clone();
                    shadow_database_url.set_path(&format!("/{}", shadow_database_name));
                    let params = ConnectorParams {
                        connection_string: shadow_database_url.to_string(),
                        preview_features: params.connector_params.preview_features,
                        shadow_database_connection_string: None,
                    };

                    let host = shadow_database_url.host();
                    tracing::debug!("Connecting to shadow database at {:?}/{}", host, shadow_database_name);
                    shadow_database.set_params(params)?;

                    // We go through the whole process without early return, then clean up
                    // the shadow database, and only then return the result. This avoids
                    // leaving shadow databases behind in case of e.g. faulty migrations.
                    let ret = shadow_db::sql_schema_from_migrations_history(migrations, shadow_database).await;

                    let drop_database = format!("DROP DATABASE IF EXISTS `{}`", shadow_database_name);
                    conn.raw_cmd(&drop_database).await?;

                    ret
                })
            }
        }
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        with_connection(&mut self.state, |_, _, connection| async {
            Ok(connection.version().await?)
        })
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

fn check_datamodel_for_mysql_5_6(datamodel: &ValidatedSchema, errors: &mut Vec<String>) {
    datamodel
        .db
        .walk_models()
        .flat_map(|model| model.scalar_fields())
        .for_each(|field| {
            if field
                .scalar_type()
                .map(|t| matches!(t, ScalarType::Json))
                .unwrap_or(false)
            {
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

        let mut flavour = MysqlFlavour::default();
        let params = ConnectorParams {
            connection_string: url.to_owned(),
            preview_features: Default::default(),
            shadow_database_connection_string: None,
        };
        flavour.set_params(params).unwrap();
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

fn with_connection<'a, O, F, C>(state: &'a mut State, f: C) -> BoxFuture<'a, ConnectorResult<O>>
where
    O: 'a,
    F: future::Future<Output = ConnectorResult<O>> + Send + 'a,
    C: (FnOnce(&'a mut Params, BitFlags<Circumstances>, &'a mut Connection) -> F) + Send + 'a,
{
    static MYSQL_SYSTEM_DATABASES: Lazy<regex::RegexSet> = Lazy::new(|| {
        RegexSet::new(&[
            "(?i)^mysql$",
            "(?i)^information_schema$",
            "(?i)^performance_schema$",
            "(?i)^sys$",
        ])
        .unwrap()
    });

    match state {
        super::State::Initial => panic!("logic error: Initial"),
        super::State::Connected(p, (circumstances, conn)) => Box::pin(f(p, *circumstances, conn)),
        state @ super::State::WithParams(_) => Box::pin(async move {
            state
                .try_connect(|params| {
                    Box::pin(async move {
                        let db_name = params.url.dbname();
                        let connection = connect(&params.connector_params.connection_string).await?;

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

                        Ok((circumstances, connection))
                    })
                })
                .await?;
            with_connection(state, f).await
        }),
    }
}

fn scan_migration_script_impl(script: &str) {
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

fn convert_server_error(circumstances: BitFlags<Circumstances>, error: &my::Error) -> Option<KnownError> {
    if circumstances.contains(Circumstances::IsVitess) {
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
