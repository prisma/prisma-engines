mod connector;
mod destructive_change_checker;
mod renderer;
mod schema_calculator;
mod schema_differ;

use crate::{flavour::SqlConnector, sql_renderer::SqlRenderer};
use connector as imp;
use destructive_change_checker::SurrealDbDestructiveChangeCheckerFlavour;
use renderer::SurrealDbRenderer;
use schema_calculator::SurrealDbSchemaCalculatorFlavour;
use schema_connector::{
    BoxFuture, ConnectorError, ConnectorResult, Namespaces, SchemaFilter, migrations_directory::Migrations,
};
use schema_differ::SurrealDbSchemaDifferFlavour;
use sql_schema_describer::SqlSchema;
use std::future::Future;

use super::{SqlDialect, UsingExternalShadowDb};

type State = imp::State;

#[derive(Debug, Default)]
pub struct SurrealDbDialect;

impl SqlDialect for SurrealDbDialect {
    fn renderer(&self) -> Box<dyn SqlRenderer> {
        Box::new(SurrealDbRenderer)
    }

    fn schema_differ(&self) -> Box<dyn crate::sql_schema_differ::SqlSchemaDifferFlavour> {
        Box::new(SurrealDbSchemaDifferFlavour)
    }

    fn schema_calculator(&self) -> Box<dyn crate::sql_schema_calculator::SqlSchemaCalculatorFlavour> {
        Box::new(SurrealDbSchemaCalculatorFlavour)
    }

    fn destructive_change_checker(
        &self,
    ) -> Box<dyn crate::sql_destructive_change_checker::DestructiveChangeCheckerFlavour> {
        Box::new(SurrealDbDestructiveChangeCheckerFlavour)
    }

    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::SURREALDB
    }

    #[cfg(not(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    )))]
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
            Ok(Box::new(SurrealDbConnector::new_external(adapter)) as Box<dyn SqlConnector>)
        })
    }

    #[cfg(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    ))]
    fn connect_to_shadow_db(
        &self,
        _url: String,
        _preview_features: psl::PreviewFeatures,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn SqlConnector>>> {
        Box::pin(async move {
            Err(ConnectorError::from_msg(
                "SurrealDB does not support native shadow databases".into(),
            ))
        })
    }
}

pub(crate) struct SurrealDbConnector {
    state: State,
}

impl SurrealDbConnector {
    fn with_connection<'a, F, O, C>(&'a mut self, f: C) -> BoxFuture<'a, ConnectorResult<O>>
    where
        O: 'a + Send,
        C: (FnOnce(&'a imp::Connection, &'a imp::Params) -> F) + Send + Sync + 'a,
        F: Future<Output = ConnectorResult<O>> + Send + 'a,
    {
        Box::pin(async move {
            let (connection, params) = imp::get_connection_and_params(&mut self.state)?;
            f(connection, params).await
        })
    }
}

impl SurrealDbConnector {
    pub(crate) fn new_external(adapter: std::sync::Arc<dyn quaint::connector::ExternalConnector>) -> Self {
        Self {
            state: State::new(adapter, Default::default()),
        }
    }
}

impl std::fmt::Debug for SurrealDbConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<SurrealDB connector>")
    }
}

impl SqlConnector for SurrealDbConnector {
    fn dialect(&self) -> Box<dyn SqlDialect> {
        Box::new(SurrealDbDialect)
    }

    fn shadow_db_url(&self) -> Option<&str> {
        imp::get_shadow_db_url(&self.state)
    }

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd(concat!(
            "DEFINE TABLE IF NOT EXISTS _prisma_lock SCHEMAFULL;",
            "DEFINE FIELD IF NOT EXISTS locked_by ON TABLE _prisma_lock TYPE string;",
            "DEFINE FIELD IF NOT EXISTS locked_at ON TABLE _prisma_lock TYPE datetime DEFAULT time::now();",
            "UPSERT _prisma_lock:migration_lock SET locked_by = 'prisma', locked_at = time::now();",
        ))
    }

    fn connector_type(&self) -> &'static str {
        "surrealdb"
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        self.with_connection(|conn, _| conn.apply_migration_script(migration_name, script))
    }

    fn table_names(
        &mut self,
        _namespaces: Option<Namespaces>,
        _filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<Vec<String>>> {
        self.with_connection(|conn, _| conn.list_tables())
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(imp::create_database(&self.state))
    }

    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd(indoc::indoc! {r#"
            DEFINE TABLE _prisma_migrations SCHEMAFULL;
            DEFINE FIELD id ON TABLE _prisma_migrations TYPE string;
            DEFINE FIELD checksum ON TABLE _prisma_migrations TYPE string;
            DEFINE FIELD finished_at ON TABLE _prisma_migrations TYPE option<datetime>;
            DEFINE FIELD migration_name ON TABLE _prisma_migrations TYPE string;
            DEFINE FIELD logs ON TABLE _prisma_migrations TYPE option<string>;
            DEFINE FIELD rolled_back_at ON TABLE _prisma_migrations TYPE option<datetime>;
            DEFINE FIELD started_at ON TABLE _prisma_migrations TYPE datetime DEFAULT time::now();
            DEFINE FIELD applied_steps_count ON TABLE _prisma_migrations TYPE int DEFAULT 0;
            DEFINE INDEX _prisma_migrations_pk ON TABLE _prisma_migrations FIELDS id UNIQUE;
        "#})
    }

    fn describe_schema(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        Box::pin(imp::introspect(&mut self.state))
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(imp::drop_database(&self.state))
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("REMOVE TABLE _prisma_migrations")
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(imp::ensure_connection_validity(&mut self.state))
    }

    fn describe_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<quaint::connector::DescribedQuery>> {
        self.with_connection(|conn, params| conn.describe_query(sql, params))
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        self.with_connection(|conn, _| conn.query(query))
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::prelude::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        self.with_connection(|conn, _| conn.query_raw(sql, params))
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        self.with_connection(|conn, _| conn.raw_cmd(sql))
    }

    fn reset(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        self.with_connection(|conn, params| conn.reset(params))
    }

    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        _namespaces: Option<Namespaces>,
        _filter: &'a SchemaFilter,
        _external_shadow_db: UsingExternalShadowDb,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        Box::pin(async move {
            let (conn, _) = imp::get_connection_and_params(&mut self.state)?;

            // Apply init script if present
            if !migrations.shadow_db_init_script.trim().is_empty() {
                conn.raw_cmd(&migrations.shadow_db_init_script).await?;
            }

            // Apply each migration in order
            for migration in migrations.migration_directories.iter() {
                let script = migration.read_migration_script()?;

                tracing::debug!(
                    "Applying migration `{}` to SurrealDB.",
                    migration.migration_name()
                );

                conn.raw_cmd(&script).await.map_err(|connector_error| {
                    connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                })?;
            }

            // Introspect the resulting schema
            imp::introspect(&mut self.state).await
        })
    }

    fn set_preview_features(&mut self, features: psl::PreviewFeatures) {
        imp::set_preview_features(&mut self.state, features);
    }

    fn preview_features(&self) -> psl::PreviewFeatures {
        imp::get_preview_features(&self.state)
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        self.with_connection(|conn, _| conn.version())
    }

    fn search_path(&self) -> &str {
        ""
    }

    fn default_namespace(&self) -> Option<&str> {
        None
    }

    fn dispose(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(imp::dispose(&self.state))
    }
}
