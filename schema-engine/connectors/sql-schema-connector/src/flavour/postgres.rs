#[cfg(feature = "postgresql-native")]
mod native;

#[cfg(not(feature = "postgresql-native"))]
mod wasm;

#[cfg(feature = "postgresql-native")]
use native as imp;

use psl::PreviewFeature;
use user_facing_errors::schema_engine::DatabaseSchemaInconsistent;
#[cfg(not(feature = "postgresql-native"))]
use wasm as imp;

use crate::SqlFlavour;
use enumflags2::BitFlags;
use indoc::indoc;
use quaint::{
    connector::{PostgresUrl, PostgresWebSocketUrl},
    Value,
};
use schema_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult, Namespaces,
};
use sql_schema_describer::{postgres::PostgresSchemaExt, SqlSchema};
use std::{
    borrow::Cow,
    future::{self, Future},
    str::FromStr,
    sync::LazyLock,
    time,
};
use url::Url;

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

#[cfg(feature = "postgresql-native")]
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
    pub(crate) async fn new_external(
        adapter: std::sync::Arc<dyn quaint::connector::ExternalConnector>,
        factory: std::sync::Arc<dyn quaint::connector::ExternalConnectorFactory>,
    ) -> ConnectorResult<Self> {
        let provider = PostgresProvider::Unspecified;
        Ok(PostgresFlavour {
            state: State::new(adapter, factory, provider, Default::default()).await?,
            provider,
        })
    }

    #[cfg(feature = "postgresql-native")]
    pub(crate) fn new_postgres() -> Self {
        PostgresFlavour {
            state: State::Initial,
            provider: PostgresProvider::PostgreSql,
        }
    }

    #[cfg(feature = "postgresql-native")]
    pub(crate) fn new_cockroach() -> Self {
        PostgresFlavour {
            state: State::Initial,
            provider: PostgresProvider::CockroachDb,
        }
    }

    #[cfg(feature = "postgresql-native")]
    pub(crate) fn new_unspecified() -> Self {
        PostgresFlavour {
            state: State::Initial,
            provider: PostgresProvider::Unspecified,
        }
    }

    fn circumstances(&self) -> Option<BitFlags<Circumstances>> {
        imp::get_circumstances(&self.state)
    }

    pub(crate) fn is_cockroachdb(&self) -> bool {
        self.provider == PostgresProvider::CockroachDb
            || self
                .circumstances()
                .is_some_and(|c| c.contains(Circumstances::IsCockroachDb))
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

impl SqlFlavour for PostgresFlavour {
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
            tokio::time::timeout(
                ADVISORY_LOCK_TIMEOUT,
                connection.raw_cmd("SELECT pg_advisory_lock(72707369)"),
            )
            .await
            .map_err(|_elapsed| imp::timeout_error(params))?
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

    fn connection_string(&self) -> Option<&str> {
        imp::get_connection_string(&self.state)
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

    fn empty_database_schema(&self) -> SqlSchema {
        let mut schema = SqlSchema::default();
        schema.set_connector_data(Box::<sql_schema_describer::postgres::PostgresSchemaExt>::default());
        schema
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

    fn set_params(&mut self, connector_params: ConnectorParams) -> ConnectorResult<()> {
        imp::set_params(&mut self.state, connector_params)
    }

    fn set_preview_features(&mut self, preview_features: enumflags2::BitFlags<psl::PreviewFeature>) {
        imp::set_preview_features(&mut self.state, preview_features)
    }

    #[tracing::instrument(skip(self, migrations))]
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        shadow_database_connection_string: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        Box::pin(imp::shadow_db::sql_schema_from_migration_history(
            &mut self.state,
            self.provider,
            migrations,
            shadow_database_connection_string,
            namespaces,
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
    use sql_schema_describer::{postgres as describer, DescriberErrorKind, SqlSchemaDescriberBackend};

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

    crate::flavour::normalize_sql_schema(&mut schema, preview_features);
    normalize_sql_schema(&mut schema, preview_features);

    Ok(schema)
}

async fn sql_schema_from_migrations_and_db(
    conn: &imp::Connection,
    params: &imp::Params,
    schema: String,
    migrations: &[MigrationDirectory],
    namespaces: Option<Namespaces>,
    circumstances: BitFlags<Circumstances>,
    preview_features: BitFlags<PreviewFeature>,
) -> ConnectorResult<SqlSchema> {
    if circumstances.contains(Circumstances::IsCockroachDb) {
        // CockroachDB is very slow in applying DDL statements.
        // A workaround to it is to run the statements in a transaction block. This comes with some
        // drawbacks and limitations though, so we only apply this when creating a shadow db.
        // See https://www.cockroachlabs.com/docs/stable/online-schema-changes#limitations
        // Original GitHub issue with context: https://github.com/prisma/prisma/issues/12384#issuecomment-1152523689
        conn.raw_cmd("BEGIN;").await.map_err(imp::quaint_error_mapper(params))?;
    }

    for migration in migrations {
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

    if circumstances.contains(Circumstances::IsCockroachDb) {
        conn.raw_cmd("COMMIT;")
            .await
            .map_err(imp::quaint_error_mapper(params))?;
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
