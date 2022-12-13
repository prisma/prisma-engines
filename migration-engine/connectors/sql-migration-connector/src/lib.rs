//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::ptr_arg)] // remove after https://github.com/rust-lang/rust-clippy/issues/8482 is fixed and shipped

mod apply_migration;
mod database_schema;
mod error;
mod flavour;
mod pair;
mod sql_destructive_change_checker;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

use database_schema::SqlDatabaseSchema;
use enumflags2::BitFlags;
use flavour::{MssqlFlavour, MysqlFlavour, PostgresFlavour, SqlFlavour, SqliteFlavour};
use migration_connector::{migrations_directory::MigrationDirectory, *};
use pair::Pair;
use psl::ValidatedSchema;
use sql_introspection_connector::datamodel_calculator;
use sql_migration::{DropUserDefinedType, DropView, SqlMigration, SqlMigrationStep};
use sql_schema_describer as sql;
use std::sync::Arc;

const MIGRATIONS_TABLE_NAME: &str = "_prisma_migrations";

/// The top-level SQL migration connector.
pub struct SqlMigrationConnector {
    flavour: Box<dyn SqlFlavour + Send + Sync + 'static>,
    host: Arc<dyn ConnectorHost>,
}

impl SqlMigrationConnector {
    /// Initialize a PostgreSQL migration connector.
    pub fn new_postgres() -> Self {
        SqlMigrationConnector {
            flavour: Box::new(PostgresFlavour::default()),
            host: Arc::new(EmptyHost),
        }
    }

    /// Initialize a CockroachDb migration connector.
    pub fn new_cockroach() -> Self {
        SqlMigrationConnector {
            flavour: Box::new(PostgresFlavour::new_cockroach()),
            host: Arc::new(EmptyHost),
        }
    }

    /// Initialize a SQLite migration connector.
    pub fn new_sqlite() -> Self {
        SqlMigrationConnector {
            flavour: Box::new(SqliteFlavour::default()),
            host: Arc::new(EmptyHost),
        }
    }

    /// Initialize a MySQL migration connector.
    pub fn new_mysql() -> Self {
        SqlMigrationConnector {
            flavour: Box::new(MysqlFlavour::default()),
            host: Arc::new(EmptyHost),
        }
    }

    /// Initialize a MSSQL migration connector.
    pub fn new_mssql() -> Self {
        SqlMigrationConnector {
            flavour: Box::new(MssqlFlavour::default()),
            host: Arc::new(EmptyHost),
        }
    }

    fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync) {
        self.flavour.as_ref()
    }

    /// Made public for tests.
    pub fn describe_schema(
        &mut self,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'_, ConnectorResult<sql::SqlSchema>> {
        self.flavour.describe_schema(namespaces)
    }

    /// For tests
    pub async fn query_raw(
        &mut self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.flavour.query_raw(sql, params).await
    }

    /// For tests
    pub async fn query(
        &mut self,
        query: impl Into<quaint::ast::Query<'_>>,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.flavour.query(query.into()).await
    }

    /// For tests
    pub async fn raw_cmd(&mut self, sql: &str) -> ConnectorResult<()> {
        self.flavour.raw_cmd(sql).await
    }

    /// Prepare the connector to connect.
    pub fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()> {
        self.flavour.set_params(params)
    }

    async fn db_schema_from_diff_target(
        &mut self,
        target: DiffTarget<'_>,
        shadow_database_connection_string: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> ConnectorResult<SqlDatabaseSchema> {
        match target {
            DiffTarget::Datamodel(schema) => {
                let schema = psl::parse_schema(schema).map_err(ConnectorError::new_schema_parser_error)?;
                self.flavour.check_schema_features(&schema)?;
                Ok(sql_schema_calculator::calculate_sql_schema(
                    &schema,
                    self.flavour.as_ref(),
                ))
            }
            DiffTarget::Migrations(migrations) => self
                .flavour
                .sql_schema_from_migration_history(migrations, shadow_database_connection_string, namespaces)
                .await
                .map(From::from),
            DiffTarget::Database => self.flavour.describe_schema(namespaces).await.map(From::from),
            DiffTarget::Empty => Ok(self.flavour.empty_database_schema().into()),
        }
    }
}

impl MigrationConnector for SqlMigrationConnector {
    fn set_host(&mut self, host: Arc<dyn migration_connector::ConnectorHost>) {
        self.host = host;
    }

    fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()> {
        self.flavour.set_params(params)
    }

    fn set_preview_features(&mut self, preview_features: BitFlags<psl::PreviewFeature>) {
        self.flavour.set_preview_features(preview_features)
    }

    fn connection_string(&self) -> Option<&str> {
        self.flavour.connection_string()
    }

    fn connector_type(&self) -> &'static str {
        self.flavour.connector_type()
    }

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(self.flavour.acquire_lock())
    }

    fn apply_migration<'a>(&'a mut self, migration: &'a Migration) -> BoxFuture<'a, ConnectorResult<u32>> {
        Box::pin(apply_migration::apply_migration(migration, self.flavour.as_mut()))
    }

    fn apply_script<'a>(&'a mut self, migration_name: &'a str, script: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(apply_migration::apply_script(migration_name, script, self))
    }

    fn empty_database_schema(&self) -> DatabaseSchema {
        DatabaseSchema::new(SqlDatabaseSchema::from(self.flavour.empty_database_schema()))
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.flavour.ensure_connection_validity()
    }

    fn host(&self) -> &Arc<dyn ConnectorHost> {
        &self.host
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            self.flavour
                .version()
                .await
                .map(|version| version.unwrap_or_else(|| "Database version information not available.".to_owned()))
        })
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        self.flavour.create_database()
    }

    fn database_schema_from_diff_target<'a>(
        &'a mut self,
        diff_target: DiffTarget<'a>,
        shadow_database_connection_string: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>> {
        Box::pin(async move {
            self.db_schema_from_diff_target(diff_target, shadow_database_connection_string, namespaces)
                .await
                .map(From::from)
        })
    }

    fn db_execute(&mut self, script: String) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async move { self.flavour.raw_cmd(&script).await })
    }

    #[tracing::instrument(skip(self, from, to))]
    fn diff(&self, from: DatabaseSchema, to: DatabaseSchema) -> Migration {
        let previous = SqlDatabaseSchema::from_erased(from);
        let next = SqlDatabaseSchema::from_erased(to);
        let steps = sql_schema_differ::calculate_steps(Pair::new(&previous, &next), self.flavour.as_ref());
        tracing::debug!(?steps, "Inferred migration steps.");

        Migration::new(SqlMigration {
            before: previous.describer_schema,
            after: next.describer_schema,
            steps,
        })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.flavour.drop_database()
    }

    fn introspect<'a>(
        &'a mut self,
        ctx: &'a IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<IntrospectionResult>> {
        Box::pin(async move {
            let mut namespace_names = ctx.datasource().namespaces.iter().map(|(s, _)| s.clone()).collect();
            let namespaces = Namespaces::from_vec(&mut namespace_names);
            let sql_schema = self.flavour.describe_schema(namespaces).await?;
            let datamodel = datamodel_calculator::calculate(&sql_schema, ctx)
                .map_err(|err| ConnectorError::from_source(err, "Introspection error"))?;
            Ok(datamodel)
        })
    }

    fn migration_file_extension(&self) -> &'static str {
        "sql"
    }

    fn migration_len(&self, migration: &Migration) -> usize {
        migration.downcast_ref::<SqlMigration>().steps.len()
    }

    fn render_script(
        &self,
        migration: &Migration,
        diagnostics: &DestructiveChangeDiagnostics,
    ) -> ConnectorResult<String> {
        apply_migration::render_script(migration, diagnostics, self.flavour())
    }

    fn reset(&mut self, soft: bool, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async move {
            if soft || self.flavour.reset(namespaces.clone()).await.is_err() {
                best_effort_reset(self.flavour.as_mut(), namespaces).await?;
            }

            Ok(())
        })
    }

    fn migration_summary(&self, migration: &Migration) -> String {
        migration.downcast_ref::<SqlMigration>().drift_summary()
    }

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        datamodel: &ValidatedSchema,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        self.flavour.check_database_version_compatibility(datamodel)
    }

    fn destructive_change_checker(&mut self) -> &mut dyn DestructiveChangeChecker {
        self
    }

    fn migration_persistence(&mut self) -> &mut dyn MigrationPersistence {
        self
    }

    #[tracing::instrument(skip(self, migrations))]
    fn validate_migrations<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(async move {
            self.flavour
                .sql_schema_from_migration_history(migrations, None, namespaces)
                .await?;
            Ok(())
        })
    }

    fn extract_namespaces(&self, schema: &DatabaseSchema) -> Option<Namespaces> {
        let sql_schema: &SqlDatabaseSchema = schema.downcast_ref();
        Namespaces::from_vec(
            &mut sql_schema
                .describer_schema
                .walk_namespaces()
                .map(|nw| String::from(nw.name()))
                .collect::<Vec<String>>(),
        )
    }
}

fn new_shadow_database_name() -> String {
    format!("prisma_migrate_shadow_db_{}", uuid::Uuid::new_v4())
}

/// Try to reset the database to an empty state. This should only be used
/// when we don't have the permissions to do a full reset.
#[tracing::instrument(skip(flavour))]
async fn best_effort_reset(
    flavour: &mut (dyn SqlFlavour + Send + Sync),
    namespaces: Option<Namespaces>,
) -> ConnectorResult<()> {
    best_effort_reset_impl(flavour, namespaces)
        .await
        .map_err(|err| err.into_soft_reset_failed_error())
}

async fn best_effort_reset_impl(
    flavour: &mut (dyn SqlFlavour + Send + Sync),
    namespaces: Option<Namespaces>,
) -> ConnectorResult<()> {
    tracing::info!("Attempting best_effort_reset");

    let source_schema = flavour.describe_schema(namespaces).await?;
    let target_schema = flavour.empty_database_schema();
    let mut steps = Vec::new();

    // We drop views here, not in the normal migration process to not
    // accidentally drop something we can't describe in the data model.
    let drop_views = source_schema
        .view_walkers()
        .filter(|view| !flavour.view_should_be_ignored(view.name()))
        .map(|vw| DropView::new(vw.id))
        .map(SqlMigrationStep::DropView);

    steps.extend(drop_views);

    let diffables: Pair<SqlDatabaseSchema> = Pair::new(source_schema, target_schema).map(From::from);
    steps.extend(sql_schema_differ::calculate_steps(diffables.as_ref(), flavour));
    let (source_schema, target_schema) = diffables.map(|s| s.describer_schema).into_tuple();

    let drop_udts = source_schema
        .udt_walkers()
        .map(|udtw| udtw.id)
        .map(DropUserDefinedType::new)
        .map(SqlMigrationStep::DropUserDefinedType);

    steps.extend(drop_udts);

    let migration = SqlMigration {
        before: source_schema,
        after: target_schema,
        steps,
    };

    if migration.before.table_walker(crate::MIGRATIONS_TABLE_NAME).is_some() {
        flavour.drop_migrations_table().await?;
    }

    if migration.steps.is_empty() {
        return Ok(());
    }

    let migration = apply_migration::render_script(
        &Migration::new(migration),
        &DestructiveChangeDiagnostics::default(),
        flavour,
    )?;

    flavour.raw_cmd(&migration).await?;

    Ok(())
}
