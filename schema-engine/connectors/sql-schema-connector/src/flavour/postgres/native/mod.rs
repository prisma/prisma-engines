//! All the quaint-wrangling for the postgres connector should happen here.

pub(super) mod shadow_db;

use std::collections::HashMap;

use enumflags2::BitFlags;
use indoc::indoc;
use psl::PreviewFeature;
use quaint::{
    connector::{self, tokio_postgres::error::ErrorPosition, MakeTlsConnectorManager, PostgresUrl},
    prelude::{ConnectionInfo, Queryable},
};
use schema_connector::{ConnectorError, ConnectorParams, ConnectorResult};
use url::Url;
use user_facing_errors::{
    common::{DatabaseAccessDenied, DatabaseDoesNotExist},
    schema_engine::{self, ApplyMigrationError},
    UserFacingError,
};

use crate::sql_renderer::IteratorJoin;

use super::{Circumstances, MigratePostgresUrl, PostgresProvider, ADVISORY_LOCK_TIMEOUT};

pub(super) type State = crate::flavour::State<Params, (BitFlags<Circumstances>, Connection)>;

#[derive(Debug, Clone)]
pub(super) struct Params {
    connector_params: ConnectorParams,
    url: MigratePostgresUrl,
}

impl Params {
    pub fn new(connector_params: ConnectorParams) -> ConnectorResult<Self> {
        let mut url: Url = connector_params
            .connection_string
            .parse()
            .map_err(ConnectorError::url_parse_error)?;
        disable_postgres_statement_cache(&mut url)?;

        let url = MigratePostgresUrl::new(url)?;
        Ok(Self { connector_params, url })
    }
}

pub(super) struct Connection(connector::PostgreSqlWithNoCache);

impl Connection {
    pub async fn new(params: &Params) -> ConnectorResult<Connection> {
        let quaint = match &params.url.0 {
            PostgresUrl::Native(ref native_url) => {
                let tls_manager = MakeTlsConnectorManager::new(native_url.as_ref().clone());
                connector::PostgreSqlWithNoCache::new(native_url.as_ref().clone(), &tls_manager).await
            }
            PostgresUrl::WebSocket(ref ws_url) => connector::PostgreSql::new_with_websocket(ws_url.clone()).await,
        }
        .map_err(quaint_error_mapper(params))?;

        let version = quaint.version().await.map_err(quaint_error_mapper(params))?;

        if let Some(version) = version {
            let cockroach_version_prefix = "CockroachDB CCL v";

            let semver: Option<(u8, u8)> = version.strip_prefix(cockroach_version_prefix).and_then(|v| {
                let semver_unparsed: String = v.chars().take_while(|&c| c.is_ascii_digit() || c == '.').collect();

                // we only consider the major and minor version, as the patch version is not interesting for us
                semver_unparsed.split_once('.').and_then(|(major, minor_and_patch)| {
                    let major = major.parse::<u8>().ok();

                    let minor = minor_and_patch
                        .chars()
                        .take_while(|&c| c != '.')
                        .collect::<String>()
                        .parse::<u8>()
                        .ok();

                    major.zip(minor)
                })
            });

            match semver {
                Some((major, minor)) if (major == 22 && minor >= 2) || major >= 23 => {
                    // we're on 22.2+ or 23+
                    //
                    // first config issue: https://github.com/prisma/prisma/issues/16909
                    // second config value: Currently at least version 22.2.5, enums are
                    // not case-sensitive without this.
                    quaint
                        .raw_cmd(indoc! {r#"
                            SET enable_implicit_transaction_for_batch_statements=false;
                            SET use_declarative_schema_changer=off
                        "#})
                        .await
                        .map_err(quaint_error_mapper(params))?;
                }
                None | Some(_) => (),
            };
        }

        Ok(Self(quaint))
    }

    pub fn as_connector(&self) -> &connector::PostgreSqlWithNoCache {
        &self.0
    }

    // Query methods return quaint::Result directly to let the caller decide how to convert
    // the error. This is needed for errors that use information related to the connection.

    pub async fn raw_cmd(&self, sql: &str) -> quaint::Result<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await
    }

    pub async fn query(&self, query: quaint::ast::Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Postgres::build(query).unwrap();
        self.query_raw(&sql, &params).await
    }

    pub async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> quaint::Result<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql, ?params);
        self.0.query_raw(sql, params).await
    }

    pub async fn version(&self) -> quaint::Result<Option<String>> {
        tracing::debug!(query_type = "version");
        self.0.version().await
    }

    pub async fn describe_query(&self, sql: &str) -> quaint::Result<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        self.0.describe_query(sql).await
    }

    pub async fn apply_migration_script(&self, migration_name: &str, script: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", script);
        let client = self.0.client();

        match client.simple_query(script).await {
            Ok(_) => Ok(()),
            Err(err) => {
                let (database_error_code, database_error): (Option<&str>, _) = if let Some(db_error) = err.as_db_error()
                {
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
                                    "\n\nPosition:\n{numbered_lines}\n\x1b[1m{line_number:>3}\x1b[1;31m {line}\x1b[0m"
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

                    let database_error = format!("{db_error}{position}\n\n{db_error:?}");

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
    }
}

pub(super) async fn create_database(state: &State) -> ConnectorResult<String> {
    let params = state.get_unwrapped_params();
    let schema_name = params.url.schema();
    let db_name = params.url.dbname();

    let (admin_conn, admin_params) = create_postgres_admin_conn(params.clone()).await?;

    let query = format!("CREATE DATABASE \"{db_name}\"");

    let mut database_already_exists_error = None;

    match admin_conn
        .raw_cmd(&query)
        .await
        .map_err(quaint_error_mapper(&admin_params))
    {
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
    let conn = Connection::new(params).await?;

    let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\";");

    conn.raw_cmd(&schema_sql).await.map_err(quaint_error_mapper(params))?;

    if let Some(err) = database_already_exists_error {
        return Err(err);
    }

    Ok(db_name.to_owned())
}

pub(super) async fn drop_database(state: &State) -> ConnectorResult<()> {
    let params = state.get_unwrapped_params();
    let db_name = params.url.dbname();
    assert!(!db_name.is_empty(), "Database name should not be empty.");

    let (admin_conn, admin_params) = create_postgres_admin_conn(params.clone()).await?;

    admin_conn
        .raw_cmd(&format!("DROP DATABASE \"{db_name}\""))
        .await
        .map_err(quaint_error_mapper(&admin_params))?;

    Ok(())
}

pub(super) fn get_circumstances(state: &State) -> Option<BitFlags<Circumstances>> {
    match state {
        State::Connected(_, (circumstances, _)) => Some(*circumstances),
        _ => None,
    }
}

pub(super) fn get_default_schema(state: &State) -> Option<&str> {
    state.params().map(|params| params.url.schema())
}

pub(super) async fn get_connection_and_params_and_circumstances(
    state: &mut State,
    provider: PostgresProvider,
) -> ConnectorResult<(&Connection, &Params, BitFlags<Circumstances>)> {
    match state {
        State::Initial => panic!("logic error: Initial"),
        State::Connected(params, (circumstances, conn)) => Ok((conn, params, *circumstances)),
        State::WithParams(params) => {
            let conn = Connection::new(params).await?;
            let circumstances = super::setup_connection(&conn, params, provider, params.url.schema()).await?;
            *state = State::Connected(params.clone(), (circumstances, conn));

            let State::Connected(params, (circumstances, conn)) = state else {
                unreachable!();
            };
            Ok((conn, params, *circumstances))
        }
    }
}

pub(super) async fn get_connection_and_params(
    state: &mut State,
    provider: PostgresProvider,
) -> ConnectorResult<(&Connection, &Params)> {
    let (conn, params, _) = get_connection_and_params_and_circumstances(state, provider).await?;
    Ok((conn, params))
}

pub(super) fn get_preview_features(state: &State) -> BitFlags<PreviewFeature> {
    state.get_unwrapped_params().connector_params.preview_features
}

pub(super) fn set_preview_features(state: &mut State, preview_features: BitFlags<PreviewFeature>) {
    match state {
        State::Initial => {
            if !preview_features.is_empty() {
                tracing::warn!("set_preview_feature on Initial state has no effect ({preview_features}).");
            }
        }
        State::WithParams(params) | State::Connected(params, _) => {
            params.connector_params.preview_features = preview_features
        }
    }
}

pub(super) fn quaint_error_mapper(params: &Params) -> impl Fn(quaint::error::Error) -> ConnectorError + use<'_> {
    |err| crate::flavour::quaint_error_to_connector_error(err, &ConnectionInfo::Native(params.url.clone().into()))
}

pub(super) fn timeout_error(params: &Params) -> ConnectorError {
    ConnectorError::user_facing(user_facing_errors::common::DatabaseTimeout {
        database_host: params.url.host().to_owned(),
        database_port: params.url.port().to_string(),
        context: format!(
            "Timed out trying to acquire a postgres advisory lock (SELECT pg_advisory_lock(72707369)). Elapsed: {}ms. See https://pris.ly/d/migrate-advisory-locking for details.", ADVISORY_LOCK_TIMEOUT.as_millis()
        ),
    })
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut params: Params) -> ConnectorResult<(Connection, Params)> {
    // "postgres" is the default database on most postgres installations,
    // "template1" is guaranteed to exist, and "defaultdb" is the only working
    // option on DigitalOcean managed postgres databases.
    const CANDIDATE_DEFAULT_DATABASES: &[&str] = &["postgres", "template1", "defaultdb"];

    let mut conn = None;

    let mut url = Url::parse(&params.connector_params.connection_string).map_err(ConnectorError::url_parse_error)?;
    strip_schema_param_from_url(&mut url);

    for database_name in CANDIDATE_DEFAULT_DATABASES {
        url.set_path(&format!("/{database_name}"));
        params.url = MigratePostgresUrl::new(url.clone())?;

        match Connection::new(&params).await {
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
                conn = Some(other_outcome.map(|conn| (conn, params)));
                break;
            }
        }
    }

    let conn = conn.ok_or_else(|| {
        ConnectorError::user_facing(schema_engine::DatabaseCreationFailed { database_error: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned() })
    })??;

    Ok(conn)
}

fn strip_schema_param_from_url(url: &mut Url) {
    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));
}

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
