mod shadow_db;

use crate::{sql_renderer::IteratorJoin, SqlFlavour};
use enumflags2::BitFlags;
use indoc::indoc;
use migration_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult,
};
use quaint::{
    connector::{tokio_postgres::error::ErrorPosition, PostgreSql as Connection, PostgresUrl},
    prelude::Queryable,
};
use sql_schema_describer::SqlSchema;
use std::{collections::HashMap, future, time};
use url::Url;
use user_facing_errors::{
    common::{DatabaseAccessDenied, DatabaseDoesNotExist},
    introspection_engine::DatabaseSchemaInconsistent,
    migration_engine::{self, ApplyMigrationError},
    UserFacingError,
};

const ADVISORY_LOCK_TIMEOUT: time::Duration = time::Duration::from_secs(10);

/// Connection settings applied to every new connection on CockroachDB.
///
/// https://www.cockroachlabs.com/docs/stable/experimental-features.html
const COCKROACHDB_PRELUDE: &str = r#"
SET enable_experimental_alter_column_type_general = true;
"#;

type State = super::State<Params, (BitFlags<Circumstances>, Connection)>;

struct Params {
    connector_params: ConnectorParams,
    url: PostgresUrl,
}

pub(crate) struct PostgresFlavour {
    state: State,
    /// Should only be set in the constructor.
    is_cockroach: bool,
}

impl Default for PostgresFlavour {
    fn default() -> Self {
        PostgresFlavour {
            state: State::Initial,
            is_cockroach: false,
        }
    }
}

impl std::fmt::Debug for PostgresFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<PostgreSQL connector>")
    }
}

impl PostgresFlavour {
    pub(crate) fn new_cockroach() -> Self {
        PostgresFlavour {
            state: State::Initial,
            is_cockroach: true,
        }
    }

    fn circumstances(&self) -> Option<BitFlags<Circumstances>> {
        match &self.state {
            State::Initial | State::WithParams(_) => None,
            State::Connected(_, (circ, _)) => Some(*circ),
        }
    }

    pub(crate) fn is_cockroachdb(&self) -> bool {
        self.is_cockroach
            || self
                .circumstances()
                .map(|c| c.contains(Circumstances::IsCockroachDb))
                .unwrap_or(false)
    }
}

impl SqlFlavour for PostgresFlavour {
    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(self, move |params, circumstances, connection| async move {
            // They do not support advisory locking:
            // https://github.com/cockroachdb/cockroach/issues/13546
            if circumstances.contains(Circumstances::IsCockroachDb) {
                return Ok(());
            }

            // https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS

            // 72707369 is a unique number we chose to identify Migrate. It does not
            // have any meaning, but it should not be used by any other tool.
            tokio::time::timeout(
                ADVISORY_LOCK_TIMEOUT,
                raw_cmd("SELECT pg_advisory_lock(72707369)", connection, &params.url),
            )
                .await
                .map_err(|_elapsed| {
                    ConnectorError::user_facing(user_facing_errors::common::DatabaseTimeout {
                        database_host: params.url.host().to_owned(),
                        database_port: params.url.port().to_string(),
                        context: format!(
                            "Timed out trying to acquire a postgres advisory lock (SELECT pg_advisory_lock(72707369)). Elapsed: {}ms. See https://pris.ly/d/migrate-advisory-locking for details.", ADVISORY_LOCK_TIMEOUT.as_millis()
                            ),
                    })
                })??;

            Ok(())
        })
    }

    fn connector_type(&self) -> &'static str {
        if self.is_cockroach {
            "cockroachdb"
        } else {
            "postgresql"
        }
    }

    fn datamodel_connector(&self) -> &'static dyn datamodel::datamodel_connector::Connector {
        if self.is_cockroachdb() {
            datamodel::builtin_connectors::COCKROACH
        } else {
            datamodel::builtin_connectors::POSTGRES
        }
    }

    fn describe_schema(&mut self) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        use sql_schema_describer::{postgres as describer, DescriberErrorKind, SqlSchemaDescriberBackend};
        with_connection(self, |params, circumstances, conn| async move {
            let mut describer_circumstances: BitFlags<describer::Circumstances> = Default::default();
            if circumstances.contains(Circumstances::IsCockroachDb) {
                describer_circumstances |= describer::Circumstances::Cockroach;
            }

            let mut schema = sql_schema_describer::postgres::SqlSchemaDescriber::new(conn, describer_circumstances)
                .describe(params.url.schema())
                .await
                .map_err(|err| match err.into_kind() {
                    DescriberErrorKind::QuaintError(err) => quaint_err(&params.url)(err),
                    e @ DescriberErrorKind::CrossSchemaReference { .. } => {
                        let err = DatabaseSchemaInconsistent {
                            explanation: e.to_string(),
                        };
                        ConnectorError::user_facing(err)
                    }
                })?;

            super::normalize_sql_schema(&mut schema, params.connector_params.preview_features);

            Ok(schema)
        })
    }

    fn query<'a>(
        &'a mut self,
        q: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(self, move |params, _, conn| query(q, conn, &params.url))
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(self, move |conn_params, _, conn| {
            query_raw(sql, params, conn, &conn_params.url)
        })
    }

    fn run_query_script<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        self.raw_cmd(sql)
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(self, move |_params, _circumstances, connection| async move {
            let client = connection.client();

            match client.simple_query(script).await {
                Ok(_) => Ok(()),
                Err(err) => {
                    let (database_error_code, database_error): (Option<&str>, _) =
                        if let Some(db_error) = err.as_db_error() {
                            let position = if let Some(ErrorPosition::Original(position)) = db_error.position() {
                                let mut previous_lines = [""; 5];
                                let mut byte_index = 0;
                                let mut error_position = String::new();

                                for (line_idx, line) in script.lines().enumerate() {
                                    // Line numbers start at 1, not 0.
                                    let line_number = line_idx + 1;
                                    byte_index += line.len() + 1; // + 1 for the \n character.

                                    if *position as usize <= byte_index {
                                        let numbered_lines = previous_lines
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(idx, line)| {
                                                line_number
                                                    .checked_sub(previous_lines.len() - idx)
                                                    .map(|idx| (idx, line))
                                            })
                                            .map(|(idx, line)| {
                                                format!(
                                                    "\x1b[1m{:>3}\x1b[0m{}{}",
                                                    idx,
                                                    if line.is_empty() { "" } else { " " },
                                                    line
                                                )
                                            })
                                            .join("\n");

                                        error_position = format!(
                                            "\n\nPosition:\n{}\n\x1b[1m{:>3}\x1b[1;31m {}\x1b[0m",
                                            numbered_lines, line_number, line
                                        );
                                        break;
                                    } else {
                                        previous_lines = [
                                            previous_lines[1],
                                            previous_lines[2],
                                            previous_lines[3],
                                            previous_lines[4],
                                            line,
                                        ];
                                    }
                                }

                                error_position
                            } else {
                                String::new()
                            };

                            let database_error = format!("{}{}\n\n{:?}", db_error, position, db_error);

                            (Some(db_error.code().code()), database_error)
                        } else {
                            (err.code().map(|c| c.code()), err.to_string())
                        };

                    Err(ConnectorError::user_facing(ApplyMigrationError {
                        migration_name: migration_name.to_owned(),
                        database_error_code: database_error_code.unwrap_or("none").to_owned(),
                        database_error,
                    }))
                }
            }
        })
    }

    fn connection_string(&self) -> Option<&str> {
        self.state
            .params()
            .map(|p| p.connector_params.connection_string.as_str())
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            let params = self.state.get_unwrapped_params();
            let connection_string = &params.connector_params.connection_string;
            let schema_name = params.url.schema();

            let mut url = Url::parse(connection_string).map_err(ConnectorError::url_parse_error)?;
            let db_name = params.url.dbname();

            strip_schema_param_from_url(&mut url);

            let (mut conn, admin_url) = create_postgres_admin_conn(url.clone()).await?;

            let query = format!("CREATE DATABASE \"{}\"", db_name);

            let mut database_already_exists_error = None;

            match raw_cmd(&query, &mut conn, &admin_url).await {
                Ok(_) => (),
                Err(err) if err.is_user_facing_error::<user_facing_errors::common::DatabaseAlreadyExists>() => {
                    database_already_exists_error = Some(err)
                }
                Err(err) if err.is_user_facing_error::<user_facing_errors::query_engine::UniqueKeyViolation>() => {
                    database_already_exists_error = Some(err)
                }
                Err(err) => return Err(err),
            };

            // Now create the schema
            let mut conn = Connection::new(params.url.clone())
                .await
                .map_err(quaint_err(&params.url))?;

            let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", schema_name);

            raw_cmd(&schema_sql, &mut conn, &params.url).await?;

            if let Some(err) = database_already_exists_error {
                return Err(err);
            }

            Ok(db_name.to_owned())
        })
    }

    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let sql = indoc! {r#"
            CREATE TABLE _prisma_migrations (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             TIMESTAMPTZ,
                migration_name          VARCHAR(255) NOT NULL,
                logs                    TEXT,
                rolled_back_at          TIMESTAMPTZ,
                started_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
                applied_steps_count     INTEGER NOT NULL DEFAULT 0
            );
        "#};

        self.raw_cmd(sql)
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async move {
            let params = self.state.get_unwrapped_params();
            let mut url =
                Url::parse(&params.connector_params.connection_string).map_err(ConnectorError::url_parse_error)?;
            let db_name = url.path().trim_start_matches('/').to_owned();
            assert!(!db_name.is_empty(), "Database name should not be empty.");

            strip_schema_param_from_url(&mut url);
            let (mut admin_conn, admin_url) = create_postgres_admin_conn(url.clone()).await?;

            raw_cmd(&format!("DROP DATABASE \"{}\"", db_name), &mut admin_conn, &admin_url).await?;

            Ok(())
        })
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(self.raw_cmd("DROP TABLE _prisma_migrations"))
    }

    fn empty_database_schema(&self) -> SqlSchema {
        let mut schema = SqlSchema::default();
        schema.set_connector_data(Box::new(sql_schema_describer::postgres::PostgresSchemaExt::default()));
        schema
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(self, |_, _, _| future::ready(Ok(())))
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(self, move |params, _circumstances, conn| async move {
            raw_cmd(sql, conn, &params.url).await
        })
    }

    fn reset(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(self, move |params, _circumstances, conn| async move {
            let schema_name = params.url.schema();

            raw_cmd(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name), conn, &params.url).await?;
            raw_cmd(&format!("CREATE SCHEMA \"{}\"", schema_name), conn, &params.url).await?;

            Ok(())
        })
    }

    fn set_params(&mut self, mut connector_params: ConnectorParams) -> ConnectorResult<()> {
        let mut url: Url = connector_params
            .connection_string
            .parse()
            .map_err(|err| ConnectorError::url_parse_error(err))?;
        disable_postgres_statement_cache(&mut url)?;
        let connection_string = url.to_string();
        let url = PostgresUrl::new(url).map_err(|err| ConnectorError::url_parse_error(err))?;
        connector_params.connection_string = connection_string;
        let params = Params { connector_params, url };
        self.state.set_params(params);
        Ok(())
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
        let mut shadow_database = PostgresFlavour::default();

        match shadow_database_connection_string {
            Some(shadow_database_connection_string) => Box::pin(async move {
                if let Some(params) = self.state.params() {
                    super::validate_connection_infos_do_not_match(
                        &shadow_database_connection_string,
                        &params.connector_params.connection_string,
                    )?;
                }

                let shadow_db_params = ConnectorParams {
                    connection_string: shadow_database_connection_string,
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
                with_connection(self, move |params, _circumstances, main_connection| async move {
                    let shadow_database_name = crate::new_shadow_database_name();

                    {
                        let create_database = format!("CREATE DATABASE \"{}\"", shadow_database_name);
                        raw_cmd(&create_database, main_connection, &params.url)
                            .await
                            .map_err(ConnectorError::from)
                            .map_err(|err| err.into_shadow_db_creation_error())?;
                    }

                    let mut shadow_database_url: Url = params
                        .connector_params
                        .connection_string
                        .parse()
                        .map_err(ConnectorError::url_parse_error)?;
                    shadow_database_url.set_path(&format!("/{}", shadow_database_name));
                    let shadow_db_params = ConnectorParams {
                        connection_string: shadow_database_url.to_string(),
                        preview_features: params.connector_params.preview_features,
                        shadow_database_connection_string: None,
                    };
                    shadow_database.set_params(shadow_db_params)?;
                    tracing::debug!("Connecting to shadow database `{}`", shadow_database_name);
                    shadow_database.ensure_connection_validity().await?;

                    // We go through the whole process without early return, then clean up
                    // the shadow database, and only then return the result. This avoids
                    // leaving shadow databases behind in case of e.g. faulty migrations.
                    let ret = shadow_db::sql_schema_from_migrations_history(migrations, shadow_database).await;

                    let drop_database = format!("DROP DATABASE IF EXISTS \"{}\"", shadow_database_name);
                    raw_cmd(&drop_database, main_connection, &params.url).await?;

                    ret
                })
            }
        }
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        with_connection(self, |params, _circumstances, connection| async move {
            connection.version().await.map_err(quaint_err(&params.url))
        })
    }
}

fn strip_schema_param_from_url(url: &mut Url) {
    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<(Connection, PostgresUrl)> {
    // "postgres" is the default database on most postgres installations,
    // "template1" is guaranteed to exist, and "defaultdb" is the only working
    // option on DigitalOcean managed postgres databases.
    const CANDIDATE_DEFAULT_DATABASES: &[&str] = &["postgres", "template1", "defaultdb"];

    let mut conn = None;

    for database_name in CANDIDATE_DEFAULT_DATABASES {
        url.set_path(&format!("/{}", database_name));
        let postgres_url = PostgresUrl::new(url.clone()).unwrap();
        match Connection::new(postgres_url.clone())
            .await
            .map_err(quaint_err(&postgres_url))
        {
            // If the database does not exist, try the next one.
            Err(err) => match &err.error_code() {
                Some(DatabaseDoesNotExist::ERROR_CODE) => (),
                Some(DatabaseAccessDenied::ERROR_CODE) => (),
                _ => {
                    conn = Some(Err(err));
                    break;
                }
            },
            // If the outcome is anything else, use this.
            other_outcome => {
                conn = Some(other_outcome.map(|conn| (conn, postgres_url)));
                break;
            }
        }
    }

    let conn = conn.ok_or_else(|| {
        ConnectorError::user_facing(migration_engine::DatabaseCreationFailed { database_error: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned() })
    })??;

    Ok(conn)
}

#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum Circumstances {
    IsCockroachDb,
}

#[allow(clippy::needless_collect)] // clippy is wrong
fn disable_postgres_statement_cache(url: &mut Url) -> ConnectorResult<()> {
    let params: Vec<(String, String)> = url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

    url.query_pairs_mut().clear();

    for (k, v) in params.into_iter() {
        if k == "statement_cache_size" {
            url.query_pairs_mut().append_pair("statement_cache_size", "0");
        } else {
            url.query_pairs_mut().append_pair(&k, &v);
        }
    }

    if !url.query_pairs().any(|(k, _)| k == "statement_cache_size") {
        url.query_pairs_mut().append_pair("statement_cache_size", "0");
    }
    Ok(())
}

fn with_connection<'a, O, F, C>(flavour: &'a mut PostgresFlavour, f: C) -> BoxFuture<'a, ConnectorResult<O>>
where
    O: 'a,
    F: future::Future<Output = ConnectorResult<O>> + Send + 'a,
    C: (FnOnce(&'a mut Params, BitFlags<Circumstances>, &'a mut Connection) -> F) + Send + 'a,
{
    Box::pin(async move {
        match flavour.state {
            super::State::Initial => panic!("logic error: Initial"),
            super::State::Connected(ref mut p, (circumstances, ref mut conn)) => {
                return f(p, circumstances, conn).await
            }
            super::State::WithParams(_) => (),
        };

        let mut circumstances = BitFlags::<Circumstances>::default();
        let provider_is_cockroachdb = flavour.is_cockroach;

        if provider_is_cockroachdb {
            circumstances |= Circumstances::IsCockroachDb;
        }

        flavour.state
                .try_connect(move |params| Box::pin(async move {
                    let mut connection = Connection::new(params.url.clone()).await.map_err(quaint_err(&params.url))?;
                    let schema_name = params.url.schema();

                    let schema_exists_result = query_raw(
                            "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1), version()",
                            &[schema_name.into()],
                            &mut connection,
                            &params.url,
                        )
                        .await?;

                    let version = schema_exists_result.get(0).and_then(|row| row.at(1)).and_then(|v| v.to_string());

                    match version {
                        Some(version) => {
                            let db_is_cockroach = version.contains("CockroachDB");

                            // We will want to validate this in the future: https://github.com/prisma/prisma/issues/13222
                            // if db_is_cockroach && !provider_is_cockroachdb  {
                            //     let msg = "You are trying to connect to a CockroachDB database, but the provider in your Prisma schema is `postgresql`. Please change it to `cockroachdb`.";

                            //     return Err(ConnectorError::from_msg(msg.to_owned()));

                            if !db_is_cockroach && provider_is_cockroachdb {
                                let msg = "You are trying to connect to a PostgreSQL database, but the provider in your Prisma schema is `cockroachdb`. Please change it to `postgresql`.";

                                return Err(ConnectorError::from_msg(msg.to_owned()));
                            } else if db_is_cockroach {
                                circumstances |= Circumstances::IsCockroachDb;
                                raw_cmd(COCKROACHDB_PRELUDE, &mut connection, &params.url).await?;
                            }
                        }
                        None => {
                            tracing::warn!("Could not determine the version of the database.")
                        }
                    }

                    if let Some(true) = schema_exists_result
                        .get(0)
                        .and_then(|row| row.at(0).and_then(|value| value.as_bool()))
                    {
                        return Ok((circumstances, connection))
                    }

                    tracing::debug!(
                            "Detected that the `{schema_name}` schema does not exist on the target database. Attempting to create it.",
                        schema_name = schema_name,
                    );

                    raw_cmd(&format!("CREATE SCHEMA \"{}\"", schema_name), &mut connection, &params.url).await?;

                    Ok((circumstances, connection))
                })).await?;
        with_connection::<O, F, C>(flavour, f).await
    })
}

async fn raw_cmd(sql: &str, conn: &mut Connection, url: &PostgresUrl) -> ConnectorResult<()> {
    conn.raw_cmd(sql).await.map_err(quaint_err(url))
}

async fn query_raw(
    sql: &str,
    params: &[quaint::Value<'_>],
    conn: &mut Connection,
    url: &PostgresUrl,
) -> ConnectorResult<quaint::prelude::ResultSet> {
    conn.query_raw(sql, params).await.map_err(quaint_err(url))
}

async fn query(
    q: quaint::ast::Query<'_>,
    conn: &mut Connection,
    url: &PostgresUrl,
) -> ConnectorResult<quaint::prelude::ResultSet> {
    conn.query(q).await.map_err(quaint_err(url))
}

fn quaint_err(url: &PostgresUrl) -> impl (Fn(quaint::error::Error) -> ConnectorError) + '_ {
    use quaint::prelude::ConnectionInfo;
    |err| super::quaint_error_to_connector_error(err, &ConnectionInfo::Postgres(url.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_impl_does_not_leak_connection_info() {
        let url = "postgresql://myname:mypassword@myserver:8765/mydbname";

        let mut flavour = PostgresFlavour::default();
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
}
