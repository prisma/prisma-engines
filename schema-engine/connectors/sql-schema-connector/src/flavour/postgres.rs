#[cfg(feature = "postgresql-native")]
mod native;

#[cfg(not(feature = "postgresql-native"))]
mod wasm;

#[cfg(feature = "postgresql-native")]
use native::{shadow_db, Connection};

#[cfg(not(feature = "postgresql-native"))]
use wasm::{shadow_db, Connection};

use crate::SqlFlavour;
use enumflags2::BitFlags;
use indoc::indoc;
use once_cell::sync::Lazy;
use quaint::{
    connector::{PostgresUrl, PostgresWebSocketUrl},
    Value,
};
use schema_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult, Namespaces,
};
use sql_schema_describer::SqlSchema;
use std::{borrow::Cow, collections::HashMap, future, str::FromStr, time};
use url::Url;
use user_facing_errors::{
    common::{DatabaseAccessDenied, DatabaseDoesNotExist},
    schema_engine, UserFacingError,
};

const ADVISORY_LOCK_TIMEOUT: time::Duration = time::Duration::from_secs(10);

/// Connection settings applied to every new connection on CockroachDB.
///
/// https://www.cockroachlabs.com/docs/stable/experimental-features.html
const COCKROACHDB_PRELUDE: &str = r#"
SET enable_experimental_alter_column_type_general = true;
"#;

type State = super::State<Params, (BitFlags<Circumstances>, Connection)>;

#[derive(Debug, Clone)]
struct MigratePostgresUrl(PostgresUrl);

static MIGRATE_WS_BASE_URL: Lazy<Cow<'static, str>> = Lazy::new(|| {
    std::env::var("PRISMA_SCHEMA_ENGINE_WS_BASE_URL")
        .map(Cow::Owned)
        .unwrap_or_else(|_| Cow::Borrowed("wss://migrations.prisma-data.net/websocket"))
});

impl MigratePostgresUrl {
    const WEBSOCKET_SCHEME: &'static str = "prisma+postgres";
    const API_KEY_PARAM: &'static str = "api_key";
    const DBNAME_PARAM: &'static str = "dbname";

    fn new(url: Url) -> ConnectorResult<Self> {
        let postgres_url = if url.scheme() == Self::WEBSOCKET_SCHEME {
            let ws_url = Url::from_str(&MIGRATE_WS_BASE_URL).map_err(ConnectorError::url_parse_error)?;
            let Some((_, api_key)) = url.query_pairs().find(|(name, _)| name == Self::API_KEY_PARAM) else {
                return Err(ConnectorError::url_parse_error(
                    "Required `api_key` query string parameter was not provided in a connection URL",
                ));
            };

            let dbname_override = url.query_pairs().find(|(name, _)| name == Self::DBNAME_PARAM);
            let mut ws_url = PostgresWebSocketUrl::new(ws_url, api_key.into_owned());
            if let Some((_, dbname_override)) = dbname_override {
                ws_url.override_db_name(dbname_override.into_owned());
            }

            Ok(PostgresUrl::WebSocket(ws_url))
        } else {
            PostgresUrl::new_native(url)
        }
        .map_err(ConnectorError::url_parse_error)?;

        Ok(Self(postgres_url))
    }

    pub(super) fn host(&self) -> &str {
        self.0.host()
    }

    pub(super) fn port(&self) -> u16 {
        self.0.port()
    }

    pub(super) fn dbname(&self) -> &str {
        self.0.dbname()
    }

    pub(super) fn schema(&self) -> &str {
        self.0.schema()
    }
}

#[cfg(feature = "postgresql-native")]
impl From<MigratePostgresUrl> for quaint::prelude::NativeConnectionInfo {
    fn from(value: MigratePostgresUrl) -> Self {
        quaint::prelude::NativeConnectionInfo::Postgres(value.0)
    }
}

struct Params {
    connector_params: ConnectorParams,
    url: MigratePostgresUrl,
}

/// The specific provider that was requested by the user.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PostgresProvider {
    /// Used when `provider = "postgresql"` was specified in the schema.
    PostgreSql,
    /// Used when `provider = "cockroachdb"` was specified in the schema.
    CockroachDb,
    /// Used when there is no schema but only the connection string to the database.
    Unspecified,
}

pub(crate) struct PostgresFlavour {
    state: State,
    provider: PostgresProvider,
}

impl Default for PostgresFlavour {
    fn default() -> Self {
        PostgresFlavour::new_unspecified()
    }
}

impl std::fmt::Debug for PostgresFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<PostgreSQL connector>")
    }
}

impl PostgresFlavour {
    #[cfg(not(feature = "postgresql-native"))]
    pub(crate) fn new_external(_adapter: std::sync::Arc<dyn quaint::connector::ExternalConnector>) -> Self {
        PostgresFlavour {
            state: State::Initial,
            provider: PostgresProvider::PostgreSql,
        }
    }

    pub(crate) fn new_postgres() -> Self {
        PostgresFlavour {
            state: State::Initial,
            provider: PostgresProvider::PostgreSql,
        }
    }

    pub(crate) fn new_cockroach() -> Self {
        PostgresFlavour {
            state: State::Initial,
            provider: PostgresProvider::CockroachDb,
        }
    }

    pub(crate) fn new_unspecified() -> Self {
        PostgresFlavour {
            state: State::Initial,
            provider: PostgresProvider::Unspecified,
        }
    }

    fn circumstances(&self) -> Option<BitFlags<Circumstances>> {
        match &self.state {
            State::Initial | State::WithParams(_) => None,
            State::Connected(_, (circ, _)) => Some(*circ),
        }
    }

    pub(crate) fn is_cockroachdb(&self) -> bool {
        self.provider == PostgresProvider::CockroachDb
            || self
                .circumstances()
                .map(|c| c.contains(Circumstances::IsCockroachDb))
                .unwrap_or(false)
    }

    pub(crate) fn is_postgres(&self) -> bool {
        self.provider == PostgresProvider::PostgreSql && !self.is_cockroachdb()
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.state.params().map(|p| p.url.schema()).unwrap_or("public")
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
                connection.raw_cmd("SELECT pg_advisory_lock(72707369)",  &params.url),
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
        match self.provider {
            PostgresProvider::PostgreSql | PostgresProvider::Unspecified => "postgresql",
            PostgresProvider::CockroachDb => "cockroachdb",
        }
    }

    fn table_names(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<Vec<String>>> {
        Box::pin(async move {
            let search_path = self.schema_name().to_string();

            let mut namespaces: Vec<_> = namespaces
                .map(|ns| ns.into_iter().map(Value::text).collect())
                .unwrap_or_default();

            namespaces.push(Value::text(search_path));

            let select = r#"
                SELECT tbl.relname AS table_name
                FROM pg_class AS tbl
                INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
                WHERE tbl.relkind = 'r' AND namespace.nspname = ANY ( $1 )
            "#;

            let rows = self.query_raw(select, &[Value::array(namespaces)]).await?;

            let table_names: Vec<String> = rows
                .into_iter()
                .flat_map(|row| row.get("table_name").and_then(|s| s.to_string()))
                .collect();

            Ok(table_names)
        })
    }

    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector {
        if self.is_cockroachdb() {
            psl::builtin_connectors::COCKROACH
        } else {
            psl::builtin_connectors::POSTGRES
        }
    }

    fn describe_schema(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        with_connection(self, |params, circumstances, conn| async move {
            conn.describe_schema(circumstances, params, namespaces).await
        })
    }

    fn introspect<'a>(
        &'a mut self,
        namespaces: Option<Namespaces>,
        ctx: &'a schema_connector::IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        with_connection(self, move |params, circumstances, conn| async move {
            let mut enriched_circumstances = circumstances;
            if circumstances.contains(Circumstances::IsCockroachDb)
                && ctx.previous_schema().connector.is_provider("postgresql")
            {
                enriched_circumstances |= Circumstances::CockroachWithPostgresNativeTypes;
            }
            conn.describe_schema(enriched_circumstances, params, namespaces).await
        })
    }

    fn query<'a>(
        &'a mut self,
        q: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(self, move |params, _, conn| conn.query(q, &params.url))
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(self, move |conn_params, _, conn| {
            conn.query_raw(sql, params, &conn_params.url)
        })
    }

    fn describe_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<quaint::connector::DescribedQuery>> {
        with_connection(self, move |conn_params, _, conn| {
            conn.describe_query(sql, &conn_params.url)
        })
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(self, move |_params, _circumstances, connection| async move {
            connection.apply_migration_script(migration_name, script).await
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

            let query = format!("CREATE DATABASE \"{db_name}\"");

            let mut database_already_exists_error = None;

            match conn.raw_cmd(&query, &admin_url).await {
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
            let mut conn = Connection::new(connection_string.parse().unwrap()).await?;

            let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\";");

            conn.raw_cmd(&schema_sql, &params.url).await?;

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

            admin_conn
                .raw_cmd(&format!("DROP DATABASE \"{db_name}\""), &admin_url)
                .await?;

            Ok(())
        })
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(self.raw_cmd("DROP TABLE _prisma_migrations"))
    }

    fn empty_database_schema(&self) -> SqlSchema {
        let mut schema = SqlSchema::default();
        schema.set_connector_data(Box::<sql_schema_describer::postgres::PostgresSchemaExt>::default());
        schema
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(self, |_, _, _| future::ready(Ok(())))
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(self, move |params, _circumstances, conn| async move {
            conn.raw_cmd(sql, &params.url).await
        })
    }

    fn reset(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(self, move |params, _circumstances, conn| async move {
            let schemas_to_reset = match namespaces {
                Some(ns) => ns.into_iter().map(Cow::Owned).collect(),
                None => vec![Cow::Borrowed(params.url.schema())],
            };

            tracing::info!(?schemas_to_reset, "Resetting schema(s)");

            for schema_name in schemas_to_reset {
                conn.raw_cmd(&format!("DROP SCHEMA \"{schema_name}\" CASCADE"), &params.url)
                    .await?;
                conn.raw_cmd(&format!("CREATE SCHEMA \"{schema_name}\""), &params.url)
                    .await?;
            }

            // Drop the migrations table in the main schema, otherwise migrate dev will not
            // perceive that as a reset, since migrations are still marked as applied.
            //
            // We don't care if this fails.
            conn.raw_cmd("DROP TABLE _prisma_migrations", &params.url).await.ok();

            Ok(())
        })
    }

    fn set_params(&mut self, mut connector_params: ConnectorParams) -> ConnectorResult<()> {
        let mut url: Url = connector_params
            .connection_string
            .parse()
            .map_err(ConnectorError::url_parse_error)?;
        disable_postgres_statement_cache(&mut url)?;
        let connection_string = url.to_string();
        let url = MigratePostgresUrl::new(url)?;
        connector_params.connection_string = connection_string;
        let params = Params { connector_params, url };
        self.state.set_params(params);
        Ok(())
    }

    fn set_preview_features(&mut self, preview_features: enumflags2::BitFlags<psl::PreviewFeature>) {
        match &mut self.state {
            super::State::Initial => {
                if !preview_features.is_empty() {
                    tracing::warn!("set_preview_feature on Initial state has no effect ({preview_features}).");
                }
            }
            super::State::WithParams(params) | super::State::Connected(params, _) => {
                params.connector_params.preview_features = preview_features
            }
        }
    }

    #[tracing::instrument(skip(self, migrations))]
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        shadow_database_connection_string: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        let shadow_database_connection_string = shadow_database_connection_string.or_else(|| {
            self.state
                .params()
                .and_then(|p| p.connector_params.shadow_database_connection_string.clone())
        });
        let mut shadow_database = if self.is_cockroachdb() {
            PostgresFlavour::new_cockroach()
        } else {
            PostgresFlavour::default()
        };

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

                if shadow_database.reset(namespaces.clone()).await.is_err() {
                    crate::best_effort_reset(&mut shadow_database, namespaces.clone()).await?;
                }

                shadow_db::sql_schema_from_migrations_history(migrations, shadow_database, namespaces).await
            }),
            None => {
                let is_postgres = self.is_postgres();
                with_connection(self, move |params, _circumstances, main_connection| async move {
                    let shadow_database_name = crate::new_shadow_database_name();

                    {
                        let create_database = format!("CREATE DATABASE \"{shadow_database_name}\"");
                        main_connection
                            .raw_cmd(&create_database, &params.url)
                            .await
                            .map_err(|err| err.into_shadow_db_creation_error())?;
                    }

                    let mut shadow_database_url: Url = params
                        .connector_params
                        .connection_string
                        .parse()
                        .map_err(ConnectorError::url_parse_error)?;

                    if shadow_database_url.scheme() == MigratePostgresUrl::WEBSOCKET_SCHEME {
                        shadow_database_url
                            .query_pairs_mut()
                            .append_pair(MigratePostgresUrl::DBNAME_PARAM, &shadow_database_name);
                    } else {
                        shadow_database_url.set_path(&format!("/{shadow_database_name}"));
                    }
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
                    let ret =
                        shadow_db::sql_schema_from_migrations_history(migrations, shadow_database, namespaces).await;

                    if is_postgres {
                        drop_db_try_force(main_connection, &params.url, &shadow_database_name).await?;
                    } else {
                        let drop_database = format!("DROP DATABASE IF EXISTS \"{shadow_database_name}\"");
                        main_connection.raw_cmd(&drop_database, &params.url).await?;
                    }

                    ret
                })
            }
        }
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        with_connection(self, |params, _circumstances, connection| async move {
            // TODO: the `url` used here isn't Wasm-compatible.
            connection.version(&params.url).await
        })
    }

    fn search_path(&self) -> &str {
        self.schema_name()
    }
}

/// Drop a database using `WITH (FORCE)` syntax.
///
/// When drop database is routed through pgbouncer, the database may still be used in other pooled connections.
/// In this case, given that we (as a user) know the database will not be used any more, we can forcefully drop
/// the database. Note that `with (force)` is added in Postgres 13, and therefore we will need to
/// fallback to the normal drop if it errors with syntax error.
///
/// TL;DR,
/// 1. pg >= 13 -> it works.
/// 2. pg < 13 -> syntax error on WITH (FORCE), and then fail with db in use if pgbouncer is used.
async fn drop_db_try_force(
    conn: &mut Connection,
    url: &MigratePostgresUrl,
    database_name: &str,
) -> ConnectorResult<()> {
    let drop_database = format!("DROP DATABASE IF EXISTS \"{database_name}\" WITH (FORCE)");
    if let Err(err) = conn.raw_cmd(&drop_database, url).await {
        if let Some(msg) = err.message() {
            if msg.contains("syntax error") {
                let drop_database_alt = format!("DROP DATABASE IF EXISTS \"{database_name}\"");
                conn.raw_cmd(&drop_database_alt, url).await?;
            } else {
                return Err(err);
            }
        } else {
            return Err(err);
        }
    }
    Ok(())
}

fn strip_schema_param_from_url(url: &mut Url) {
    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<(Connection, MigratePostgresUrl)> {
    // "postgres" is the default database on most postgres installations,
    // "template1" is guaranteed to exist, and "defaultdb" is the only working
    // option on DigitalOcean managed postgres databases.
    const CANDIDATE_DEFAULT_DATABASES: &[&str] = &["postgres", "template1", "defaultdb"];

    let mut conn = None;

    for database_name in CANDIDATE_DEFAULT_DATABASES {
        url.set_path(&format!("/{database_name}"));
        let postgres_url = MigratePostgresUrl(PostgresUrl::new_native(url.clone()).unwrap());
        match Connection::new(url.clone()).await {
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
        ConnectorError::user_facing(schema_engine::DatabaseCreationFailed { database_error: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned() })
    })??;

    Ok(conn)
}

#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum Circumstances {
    IsCockroachDb,
    CockroachWithPostgresNativeTypes, // FIXME: we should really break and remove this
    CanPartitionTables,
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
        let provider = flavour.provider;

        if provider == PostgresProvider::CockroachDb {
            circumstances |= Circumstances::IsCockroachDb;
        }

        flavour.state
                .try_connect(move |params| Box::pin(async move {
                    let mut connection = Connection::new(params.connector_params.connection_string.parse().unwrap()).await?;
                    let schema_name = params.url.schema();

                    let schema_exists_result = connection.query_raw(
                            "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1), version(), current_setting('server_version_num')::integer as numeric_version;",
                            &[schema_name.into()],
                            &params.url,
                        )
                        .await?;

                    let version =
                        schema_exists_result
                          .first()
                          .and_then(|row| row.at(1).and_then(|ver_str| row.at(2).map(|ver_num| (ver_str, ver_num))))
                          .and_then(|(ver_str,ver_num)| ver_str.to_string().and_then(|version| ver_num.as_integer().map(|version_number| (version, version_number))));

                    match version {
                        Some((version, version_num)) => {
                            let db_is_cockroach = version.contains("CockroachDB");

                            if db_is_cockroach && provider == PostgresProvider::PostgreSql  {
                                let msg = "You are trying to connect to a CockroachDB database, but the provider in your Prisma schema is `postgresql`. Please change it to `cockroachdb`.";

                                return Err(ConnectorError::from_msg(msg.to_owned()));
                            }

                            if !db_is_cockroach && provider == PostgresProvider::CockroachDb {
                                let msg = "You are trying to connect to a PostgreSQL database, but the provider in your Prisma schema is `cockroachdb`. Please change it to `postgresql`.";

                                return Err(ConnectorError::from_msg(msg.to_owned()));
                            }

                            if db_is_cockroach {
                                circumstances |= Circumstances::IsCockroachDb;
                                connection.raw_cmd(COCKROACHDB_PRELUDE, &params.url).await?;
                            } else if version_num >= 100000 {
                                circumstances |= Circumstances:: CanPartitionTables;
                            }
                        }
                        None => {
                            tracing::warn!("Could not determine the version of the database.")
                        }
                    }

                    if let Some(true) = schema_exists_result
                        .first()
                        .and_then(|row| row.at(0).and_then(|value| value.as_bool()))
                    {
                        return Ok((circumstances, connection))
                    }

                    tracing::debug!(
                            "Detected that the `{schema_name}` schema does not exist on the target database. Attempting to create it.",
                        schema_name = schema_name,
                    );

                    connection.raw_cmd(&format!("CREATE SCHEMA \"{schema_name}\""), &params.url).await?;

                    Ok((circumstances, connection))
                })).await?;
        with_connection::<O, F, C>(flavour, f).await
    })
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
        let debugged = format!("{flavour:?}");

        let words = &["myname", "mypassword", "myserver", "8765", "mydbname"];

        for word in words {
            assert!(!debugged.contains(word));
        }
    }
}
