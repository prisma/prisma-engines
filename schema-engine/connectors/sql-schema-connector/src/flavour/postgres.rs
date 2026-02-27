mod connection_string;
mod connector;
mod destructive_change_checker;
mod renderer;
mod schema_calculator;
mod schema_differ;

use base64::prelude::*;
use connector as imp;
use destructive_change_checker::PostgresDestructiveChangeCheckerFlavour;
use enumflags2::BitFlags;
use indoc::indoc;
use psl::PreviewFeature;
use quaint::{
    Value,
    connector::{DEFAULT_POSTGRES_SCHEMA, PostgresUrl, PostgresWebSocketUrl, is_url_localhost},
};
use renderer::PostgresRenderer;
use schema_calculator::PostgresSchemaCalculatorFlavour;
use schema_connector::{
    BoxFuture, ConnectorError, ConnectorResult, Namespaces, SchemaFilter, migrations_directory::Migrations,
};
use schema_differ::PostgresSchemaDifferFlavour;
use serde::Deserialize;
use sql_schema_describer::{SqlSchema, postgres::PostgresSchemaExt};
use std::{
    borrow::Cow,
    future::{self, Future},
    str::FromStr,
    sync::LazyLock,
    time,
};
use url::Url;
use user_facing_errors::schema_engine::DatabaseSchemaInconsistent;

use super::{SqlConnector, SqlDialect, UsingExternalShadowDb};

const ADVISORY_LOCK_TIMEOUT: time::Duration = time::Duration::from_secs(10);

/// Connection settings applied to every new connection on CockroachDB.
///
/// https://www.cockroachlabs.com/docs/stable/experimental-features.html
const COCKROACHDB_PRELUDE: &str = r#"
SET enable_experimental_alter_column_type_general = true;
"#;

type State = imp::State;

#[derive(Debug, Clone)]
struct MigratePostgresUrl(PostgresUrl);

static MIGRATE_WS_BASE_URL: LazyLock<Cow<'static, str>> = LazyLock::new(|| {
    std::env::var("PRISMA_SCHEMA_ENGINE_WS_BASE_URL")
        .map(Cow::Owned)
        .unwrap_or_else(|_| Cow::Borrowed("wss://migrations.prisma-data.net/websocket"))
});

#[derive(Default)]
struct PpgParams<'a> {
    /// `api_key` parameter in the Prisma Postgres URL.
    ///
    /// In remote Prisma Postgres URLs, this parameter is used to authenticate the connection.
    ///
    /// In local Prisma Postgres URLs, this parameter is a base64url-encoded JSON
    /// object that contains data necessary for local PPg emulation. Schema
    /// engine decodes it to extract the connection string to the underlying
    /// PostgreSQL database to perform migrations.
    api_key: Option<Cow<'a, str>>,

    /// Database name override. Used for creating shadow databases with remote Prisma Postgres.
    db_name_override: Option<Cow<'a, str>>,
}

impl<'a> PpgParams<'a> {
    const API_KEY_PARAM: &'static str = "api_key";
    const DB_NAME_PARAM: &'static str = "dbname";

    fn parse_from(url: &'a Url) -> Result<Self, ConnectorError> {
        let mut params = Self::default();

        for (name, value) in url.query_pairs() {
            match name.as_ref() {
                Self::API_KEY_PARAM => params.api_key = Some(value),
                Self::DB_NAME_PARAM => params.db_name_override = Some(value),
                _ => {}
            }
        }

        Ok(params)
    }

    fn api_key(&self) -> ConnectorResult<&str> {
        self.api_key
            .as_deref()
            .ok_or_else(|| Self::required_param_error(Self::API_KEY_PARAM))
    }

    pub fn local_database_url(&self) -> ConnectorResult<Url> {
        self.with_local_api_key(|key| connection_string::parse(key.database_url))
    }

    pub fn local_shadow_database_url(&self) -> ConnectorResult<Url> {
        self.with_local_api_key(|key| connection_string::parse(key.shadow_database_url))
    }

    fn with_local_api_key<A>(&self, f: impl Fn(LocalPpgApiKey<'_>) -> ConnectorResult<A>) -> ConnectorResult<A> {
        let api_key_param = self.api_key()?;
        let api_key_json = BASE64_URL_SAFE_NO_PAD
            .decode(api_key_param)
            .map_err(ConnectorError::url_parse_error)?;
        let api_key: LocalPpgApiKey<'_> =
            serde_json::from_slice(&api_key_json).map_err(ConnectorError::url_parse_error)?;
        f(api_key)
    }

    fn db_name_override(&self) -> Option<&str> {
        self.db_name_override.as_deref()
    }

    fn required_param_error(param_name: &str) -> ConnectorError {
        ConnectorError::url_parse_error(format!(
            "Required `{param_name}` query string parameter was not provided in a connection URL"
        ))
    }
}

/// The contents of the JSON payload in the `api_key` query string parameter
/// in local PPg connection string.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalPpgApiKey<'a> {
    database_url: &'a str,
    shadow_database_url: &'a str,
}

impl MigratePostgresUrl {
    const PRISMA_POSTGRES_SCHEME: &'static str = "prisma+postgres";

    fn new(url: Url) -> ConnectorResult<Self> {
        let postgres_url = match url.scheme() {
            // Local Prisma Postgres
            Self::PRISMA_POSTGRES_SCHEME if is_url_localhost(&url) => {
                let params = PpgParams::parse_from(&url)?;
                let database_url = params.local_database_url()?;
                PostgresUrl::new_native(database_url).map_err(ConnectorError::url_parse_error)?
            }

            // Remote Prisma Postgres
            Self::PRISMA_POSTGRES_SCHEME => {
                let params = PpgParams::parse_from(&url).map_err(ConnectorError::url_parse_error)?;
                let ws_url = Url::from_str(&MIGRATE_WS_BASE_URL).map_err(ConnectorError::url_parse_error)?;

                let mut ws_url = PostgresWebSocketUrl::new(ws_url, params.api_key()?.to_owned());

                if let Some(dbname_override) = params.db_name_override() {
                    ws_url.override_db_name(dbname_override.to_owned());
                }

                PostgresUrl::WebSocket(ws_url)
            }

            // Generic PostgreSQL database
            _ => PostgresUrl::new_native(url).map_err(ConnectorError::url_parse_error)?,
        };

        Ok(Self(postgres_url))
    }

    pub(super) fn dbname(&self) -> Cow<'_, str> {
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

#[derive(Debug, Default)]
pub struct PostgresDialect {
    circumstances: BitFlags<Circumstances>,
}

impl PostgresDialect {
    fn new(circumstances: BitFlags<Circumstances>) -> Self {
        Self { circumstances }
    }

    pub fn cockroach() -> Self {
        Self::new(Circumstances::IsCockroachDb.into())
    }

    fn is_cockroachdb(&self) -> bool {
        self.circumstances.contains(Circumstances::IsCockroachDb)
    }
}

impl SqlDialect for PostgresDialect {
    fn renderer(&self) -> Box<dyn crate::sql_renderer::SqlRenderer> {
        Box::new(PostgresRenderer::new(self.is_cockroachdb()))
    }

    fn schema_differ(&self) -> Box<dyn crate::sql_schema_differ::SqlSchemaDifferFlavour> {
        Box::new(PostgresSchemaDifferFlavour::new(self.circumstances))
    }

    fn schema_calculator(&self) -> Box<dyn crate::sql_schema_calculator::SqlSchemaCalculatorFlavour> {
        Box::new(PostgresSchemaCalculatorFlavour::new(self.circumstances))
    }

    fn destructive_change_checker(
        &self,
    ) -> Box<dyn crate::sql_destructive_change_checker::DestructiveChangeCheckerFlavour> {
        Box::new(PostgresDestructiveChangeCheckerFlavour::new(self.circumstances))
    }

    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector {
        if self.is_cockroachdb() {
            psl::builtin_connectors::COCKROACH
        } else {
            psl::builtin_connectors::POSTGRES
        }
    }

    fn empty_database_schema(&self) -> SqlSchema {
        let mut schema = SqlSchema::default();
        schema.set_connector_data(Box::<sql_schema_describer::postgres::PostgresSchemaExt>::default());
        schema
    }

    fn default_namespace(&self) -> Option<&str> {
        Some(DEFAULT_POSTGRES_SCHEMA)
    }

    #[cfg(feature = "postgresql-native")]
    fn connect_to_shadow_db(
        &self,
        url: String,
        preview_features: psl::PreviewFeatures,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn SqlConnector>>> {
        let params = schema_connector::ConnectorParams::new(url, preview_features, None);
        Box::pin(async move { Ok(Box::new(PostgresConnector::new_with_params(params)?) as Box<dyn SqlConnector>) })
    }

    #[cfg(not(feature = "postgresql-native"))]
    fn connect_to_shadow_db(
        &self,
        factory: std::sync::Arc<dyn quaint::connector::ExternalConnectorFactory>,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn SqlConnector>>> {
        Box::pin(async move {
            let adapter = factory
                .connect_to_shadow_db()
                .await
                .ok_or_else(|| ConnectorError::from_msg("Provided adapter does not support shadow databases".into()))?
                .map_err(|e| ConnectorError::from_source(e, "Failed to connect to the shadow database"))?;
            Ok(Box::new(PostgresConnector::new_external(adapter).await?) as Box<dyn SqlConnector>)
        })
    }
}

pub(crate) struct PostgresConnector {
    state: State,
    provider: PostgresProvider,
}

impl std::fmt::Debug for PostgresConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<PostgreSQL connector>")
    }
}

impl PostgresConnector {
    #[cfg(not(feature = "postgresql-native"))]
    pub(crate) async fn new_external(
        adapter: std::sync::Arc<dyn quaint::connector::ExternalConnector>,
    ) -> ConnectorResult<Self> {
        let provider = PostgresProvider::Unspecified;
        Ok(PostgresConnector {
            state: State::new(adapter, provider, Default::default()).await?,
            provider,
        })
    }

    #[cfg(feature = "postgresql-native")]
    pub(crate) fn new_postgres(params: schema_connector::ConnectorParams) -> ConnectorResult<Self> {
        Ok(Self {
            state: State::WithParams(imp::Params::new(params)?),
            provider: PostgresProvider::PostgreSql,
        })
    }

    #[cfg(feature = "postgresql-native")]
    pub(crate) fn new_cockroach(params: schema_connector::ConnectorParams) -> ConnectorResult<Self> {
        Ok(Self {
            state: State::WithParams(imp::Params::new(params)?),
            provider: PostgresProvider::CockroachDb,
        })
    }

    #[cfg(feature = "postgresql-native")]
    pub(crate) fn new_with_params(params: schema_connector::ConnectorParams) -> ConnectorResult<Self> {
        Ok(Self {
            state: State::WithParams(imp::Params::new(params)?),
            provider: PostgresProvider::Unspecified,
        })
    }

    fn circumstances(&self) -> BitFlags<Circumstances> {
        let mut circumstances = imp::get_circumstances(&self.state).unwrap_or_default();
        if self.provider == PostgresProvider::CockroachDb {
            circumstances |= Circumstances::IsCockroachDb;
        }
        circumstances
    }

    pub(crate) fn is_cockroachdb(&self) -> bool {
        self.circumstances().contains(Circumstances::IsCockroachDb)
    }

    fn schema_name(&self) -> &str {
        imp::get_default_schema(&self.state)
    }

    fn with_connection<'a, F, O, C>(&'a mut self, f: C) -> BoxFuture<'a, ConnectorResult<O>>
    where
        O: 'a + Send,
        C: (FnOnce(&'a imp::Connection, &'a imp::Params) -> F) + Send + Sync + 'a,
        F: Future<Output = ConnectorResult<O>> + Send + 'a,
    {
        Box::pin(async move {
            let (conn, ctx) = imp::get_connection_and_params(&mut self.state, self.provider).await?;
            f(conn, ctx).await
        })
    }
}

impl SqlConnector for PostgresConnector {
    fn dialect(&self) -> Box<dyn SqlDialect> {
        Box::new(PostgresDialect::new(self.circumstances()))
    }

    fn shadow_db_url(&self) -> Option<&str> {
        imp::get_shadow_db_url(&self.state)
    }

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        // They do not support advisory locking:
        // https://github.com/cockroachdb/cockroach/issues/13546
        if self.is_cockroachdb() {
            return Box::pin(async { Ok(()) });
        }

        self.with_connection(|connection, params| async {
            // https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS

            // 72707369 is a unique number we chose to identify Migrate. It does not
            // have any meaning, but it should not be used by any other tool.
            crosstarget_utils::time::timeout(
                ADVISORY_LOCK_TIMEOUT,
                connection.raw_cmd("SELECT pg_advisory_lock(72707369)"),
            )
            .await
            .map_err(|_| ConnectorError::user_facing(user_facing_errors::common::DatabaseTimeout {
                context: format!(
                    "Timed out trying to acquire a postgres advisory lock (SELECT pg_advisory_lock(72707369)). Timeout: {}ms. See https://pris.ly/d/migrate-advisory-locking for details.", ADVISORY_LOCK_TIMEOUT.as_millis()
                ),
            }))?
            .map_err(imp::quaint_error_mapper(params))?;

            Ok(())
        })
    }

    fn connector_type(&self) -> &'static str {
        match self.provider {
            PostgresProvider::PostgreSql | PostgresProvider::Unspecified => "postgresql",
            PostgresProvider::CockroachDb => "cockroachdb",
        }
    }

    fn table_names(
        &mut self,
        namespaces: Option<Namespaces>,
        filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<Vec<String>>> {
        Box::pin(async move {
            let search_path = self.schema_name().to_string();

            let mut namespaces: Vec<_> = namespaces
                .map(|ns| ns.into_iter().map(Value::text).collect())
                .unwrap_or_default();

            namespaces.push(Value::text(search_path));

            let select = r#"
                SELECT tbl.relname AS table_name, namespace.nspname AS table_namespace
                FROM pg_class AS tbl
                INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
                WHERE tbl.relkind = 'r' AND namespace.nspname = ANY ( $1 )
            "#;

            let rows = self.query_raw(select, &[Value::array(namespaces)]).await?;

            let table_names: Vec<String> = rows
                .into_iter()
                .flat_map(|row| {
                    let ns = row.get("table_namespace").and_then(|s| s.to_string());
                    let table_name = row.get("table_name").and_then(|s| s.to_string());

                    ns.and_then(|ns| table_name.map(|table_name| (ns, table_name)))
                })
                .filter(|(ns, table_name)| {
                    !self
                        .dialect()
                        .schema_differ()
                        .contains_table(&filters.external_tables, Some(ns), table_name)
                })
                .map(|(_, table_name)| table_name)
                .collect();

            Ok(table_names)
        })
    }

    fn describe_schema(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        Box::pin(async {
            let schema = self.schema_name().to_owned();
            let preview_features = imp::get_preview_features(&self.state);
            let (conn, params, circumstances) =
                imp::get_connection_and_params_and_circumstances(&mut self.state, self.provider).await?;
            describe_schema_with(conn, params, circumstances, preview_features, namespaces, schema).await
        })
    }

    fn introspect<'a>(
        &'a mut self,
        namespaces: Option<Namespaces>,
        ctx: &'a schema_connector::IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        Box::pin(async {
            let schema = self.schema_name().to_owned();
            let preview_features = imp::get_preview_features(&self.state);
            let (conn, params, circumstances) =
                imp::get_connection_and_params_and_circumstances(&mut self.state, self.provider).await?;
            let mut enriched_circumstances = circumstances;
            if circumstances.contains(Circumstances::IsCockroachDb)
                && ctx.previous_schema().connector.is_provider("postgresql")
            {
                enriched_circumstances |= Circumstances::CockroachWithPostgresNativeTypes;
            }
            describe_schema_with(
                conn,
                params,
                enriched_circumstances,
                preview_features,
                namespaces,
                schema,
            )
            .await
        })
    }

    fn query<'a>(
        &'a mut self,
        q: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        self.with_connection(|conn, ctx| async { conn.query(q).await.map_err(imp::quaint_error_mapper(ctx)) })
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        self.with_connection(|conn, ctx| async {
            conn.query_raw(sql, params).await.map_err(imp::quaint_error_mapper(ctx))
        })
    }

    fn describe_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<quaint::connector::DescribedQuery>> {
        self.with_connection(|conn, ctx| async {
            conn.describe_query(sql).await.map_err(imp::quaint_error_mapper(ctx))
        })
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        self.with_connection(|conn, _| async { conn.apply_migration_script(migration_name, script).await })
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(imp::create_database(&self.state))
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(imp::drop_database(&self.state))
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

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("DROP TABLE _prisma_migrations")
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.with_connection(|_, _| future::ready(Ok(())))
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        self.with_connection(|conn, params| async { conn.raw_cmd(sql).await.map_err(imp::quaint_error_mapper(params)) })
    }

    fn reset(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        let default_schema = self.schema_name().to_owned();

        self.with_connection(|conn, params| async {
            let schemas_to_reset = match namespaces {
                Some(ns) => ns.into_iter().map(Cow::Owned).collect(),
                None => vec![default_schema.into()],
            };

            tracing::info!(?schemas_to_reset, "Resetting schema(s)");

            for schema_name in schemas_to_reset {
                conn.raw_cmd(&format!("DROP SCHEMA \"{schema_name}\" CASCADE"))
                    .await
                    .map_err(imp::quaint_error_mapper(params))?;
                conn.raw_cmd(&format!("CREATE SCHEMA \"{schema_name}\""))
                    .await
                    .map_err(imp::quaint_error_mapper(params))?;
            }

            // Drop the migrations table in the main schema, otherwise migrate dev will not
            // perceive that as a reset, since migrations are still marked as applied.
            //
            // We don't care if this fails.
            conn.raw_cmd("DROP TABLE _prisma_migrations").await.ok();

            Ok(())
        })
    }

    fn set_preview_features(&mut self, preview_features: enumflags2::BitFlags<psl::PreviewFeature>) {
        imp::set_preview_features(&mut self.state, preview_features)
    }

    fn preview_features(&self) -> psl::PreviewFeatures {
        imp::get_preview_features(&self.state)
    }

    #[tracing::instrument(skip(self, migrations))]
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        external_shadow_db: UsingExternalShadowDb,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        Box::pin(imp::shadow_db::sql_schema_from_migration_history(
            self,
            self.provider,
            migrations,
            namespaces,
            filter,
            external_shadow_db,
        ))
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        self.with_connection(|connection, params| async {
            connection.version().await.map_err(imp::quaint_error_mapper(params))
        })
    }

    fn search_path(&self) -> &str {
        self.schema_name()
    }

    fn default_namespace(&self) -> Option<&str> {
        Some(self.schema_name())
    }

    fn dispose(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        // Clippy thinks `imp::dispose` takes a shared reference for whatever reason.
        #[allow(clippy::unnecessary_mut_passed)]
        Box::pin(imp::dispose(&mut self.state))
    }
}

#[tracing::instrument(skip(conn, params))]
async fn describe_schema_with(
    conn: &imp::Connection,
    params: &imp::Params,
    circumstances: BitFlags<Circumstances>,
    preview_features: BitFlags<PreviewFeature>,
    namespaces: Option<Namespaces>,
    schema: String,
) -> ConnectorResult<SqlSchema> {
    use sql_schema_describer::{DescriberErrorKind, SqlSchemaDescriberBackend, postgres as describer};

    let mut describer_circumstances: BitFlags<describer::Circumstances> = Default::default();

    if circumstances.contains(Circumstances::IsCockroachDb) {
        describer_circumstances |= describer::Circumstances::Cockroach;
    }

    if circumstances.contains(Circumstances::CockroachWithPostgresNativeTypes) {
        describer_circumstances |= describer::Circumstances::CockroachWithPostgresNativeTypes;
    }

    if circumstances.contains(Circumstances::CanPartitionTables) {
        describer_circumstances |= describer::Circumstances::CanPartitionTables;
    }
    let namespaces_vec = Namespaces::to_vec(namespaces, schema);
    let namespaces_str: Vec<&str> = namespaces_vec.iter().map(AsRef::as_ref).collect();

    let mut schema =
        sql_schema_describer::postgres::SqlSchemaDescriber::new(conn.as_connector(), describer_circumstances)
            .describe(namespaces_str.as_slice())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => imp::quaint_error_mapper(params)(err),
                e @ DescriberErrorKind::CrossSchemaReference { .. } => {
                    let err = DatabaseSchemaInconsistent {
                        explanation: e.to_string(),
                    };
                    ConnectorError::user_facing(err)
                }
            })?;

    normalize_sql_schema(&mut schema, preview_features);

    Ok(schema)
}

async fn sql_schema_from_migrations_and_db(
    conn: &imp::Connection,
    params: &imp::Params,
    schema: String,
    migrations: &Migrations,
    namespaces: Option<Namespaces>,
    circumstances: BitFlags<Circumstances>,
    preview_features: BitFlags<PreviewFeature>,
) -> ConnectorResult<SqlSchema> {
    if !migrations.shadow_db_init_script.trim().is_empty() {
        conn.raw_cmd(&migrations.shadow_db_init_script)
            .await
            .map_err(imp::quaint_error_mapper(params))?;
    }

    for migration in migrations.migration_directories.iter() {
        let script = migration.read_migration_script()?;

        tracing::debug!(
            "Applying migration `{}` to shadow database.",
            migration.migration_name()
        );

        conn.raw_cmd(&script)
            .await
            .map_err(imp::quaint_error_mapper(params))
            .map_err(|connector_error| {
                connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
            })?;
    }

    describe_schema_with(conn, params, circumstances, preview_features, namespaces, schema).await
}

#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum Circumstances {
    IsCockroachDb,
    CockroachWithPostgresNativeTypes, // FIXME: we should really break and remove this
    CanPartitionTables,
}

async fn setup_connection(
    connection: &imp::Connection,
    params: &imp::Params,
    provider: PostgresProvider,
    schema_name: &str,
) -> ConnectorResult<BitFlags<Circumstances>> {
    let mut circumstances = BitFlags::<Circumstances>::default();

    if provider == PostgresProvider::CockroachDb {
        circumstances |= Circumstances::IsCockroachDb;
    }

    let schema_exists_result = connection.query_raw(
        "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1), version(), current_setting('server_version_num')::integer as numeric_version;",
        &[schema_name.into()],
    )
    .await.map_err(imp::quaint_error_mapper(params))?;

    let version = schema_exists_result
        .first()
        .and_then(|row| {
            row.at(1)
                .and_then(|ver_str| row.at(2).map(|ver_num| (ver_str, ver_num)))
        })
        .and_then(|(ver_str, ver_num)| {
            ver_str
                .to_string()
                .and_then(|version| ver_num.as_integer().map(|version_number| (version, version_number)))
        });

    match version {
        Some((version, version_num)) => {
            let db_is_cockroach = version.contains("CockroachDB");

            if db_is_cockroach && provider == PostgresProvider::PostgreSql {
                let msg = "You are trying to connect to a CockroachDB database, but the provider in your Prisma schema is `postgresql`. Please change it to `cockroachdb`.";

                return Err(ConnectorError::from_msg(msg.to_owned()));
            }

            if !db_is_cockroach && provider == PostgresProvider::CockroachDb {
                let msg = "You are trying to connect to a PostgreSQL database, but the provider in your Prisma schema is `cockroachdb`. Please change it to `postgresql`.";

                return Err(ConnectorError::from_msg(msg.to_owned()));
            }

            if db_is_cockroach {
                circumstances |= Circumstances::IsCockroachDb;
                connection
                    .raw_cmd(COCKROACHDB_PRELUDE)
                    .await
                    .map_err(imp::quaint_error_mapper(params))?;
            } else if version_num >= 100000 {
                circumstances |= Circumstances::CanPartitionTables;
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
        return Ok(circumstances);
    }

    tracing::debug!(
        "Detected that the `{schema_name}` schema does not exist on the target database. Attempting to create it.",
        schema_name = schema_name,
    );

    connection
        .raw_cmd(&format!("CREATE SCHEMA \"{schema_name}\""))
        .await
        .map_err(imp::quaint_error_mapper(params))?;

    Ok(circumstances)
}

fn normalize_sql_schema(schema: &mut SqlSchema, preview_features: BitFlags<PreviewFeature>) {
    if !preview_features.contains(PreviewFeature::PostgresqlExtensions) {
        let pg_ext: &mut PostgresSchemaExt = schema.downcast_connector_data_mut();
        pg_ext.clear_extensions();
    }
}

#[cfg(test)]
mod tests {
    use schema_connector::ConnectorParams;

    use super::*;

    #[cfg(feature = "postgresql-native")]
    #[test]
    fn debug_impl_does_not_leak_connection_info() {
        let url = "postgresql://myname:mypassword@myserver:8765/mydbname";

        let params = ConnectorParams::new(url.to_owned(), Default::default(), None);
        let connector = PostgresConnector::new_with_params(params).unwrap();
        let debugged = format!("{connector:?}");

        let words = &["myname", "mypassword", "myserver", "8765", "mydbname"];

        for word in words {
            assert!(!debugged.contains(word));
        }
    }
}
