//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::trivial_regex)] // these will grow
#![allow(clippy::redundant_closure)] // too eager, sometimes wrong

mod apply_migration;
mod connection_wrapper;
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
use datamodel::ValidatedSchema;
use flavour::{MssqlFlavour, MysqlFlavour, PostgresFlavour, SqlFlavour, SqliteFlavour};
use migration_connector::{migrations_directory::MigrationDirectory, *};
use pair::Pair;
use sql_migration::{DropUserDefinedType, DropView, SqlMigration, SqlMigrationStep};
use sql_schema_describer::{self as describer, walkers::SqlSchemaExt, SqlSchema};
use std::sync::Arc;

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

    //     /// Construct and initialize the SQL migration connector.
    //     pub fn new(params: ConnectorParams) -> ConnectorResult<Self> {
    //         let connection_info = ConnectionInfo::from_url(&params.connection_string).map_err(|err| {
    //             let details = user_facing_errors::quaint::invalid_connection_string_description(&err.to_string());
    //             KnownError::new(user_facing_errors::common::InvalidConnectionString { details })
    //         })?;

    //         let flavour = flavour::from_connection_info(&connection_info);

    //         Ok(Self {
    //             params,
    //             connection: tokio::sync::OnceCell::new(),
    //             flavour,
    //             host: Arc::new(EmptyHost),
    //         })
    //     }

    // async fn conn(&self) -> ConnectorResult<&Connection> {
    //     self.flavour.connection().await?;

    //     // self.connection
    //     //     .get_or_init(|| {
    //     //         Box::pin(async {
    //     //             let connection = connect(&self.params.connection_string).await?;
    //     //             self.flavour.ensure_connection_validity().await?;
    //     //             Ok(connection)
    //     //         })
    //     //     })
    //     //     .await
    //     //     .as_ref()
    //     //     .map_err(|err| err.clone())
    // }

    fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync) {
        self.flavour.as_ref()
    }

    /// Made public for tests.
    pub fn describe_schema(&mut self) -> BoxFuture<'_, ConnectorResult<describer::SqlSchema>> {
        self.flavour.describe_schema()
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

    async fn db_schema_from_diff_target(&mut self, target: &DiffTarget<'_>) -> ConnectorResult<SqlDatabaseSchema> {
        match target {
            DiffTarget::Datamodel(schema) => {
                let schema =
                    datamodel::parse_schema_parserdb(schema).map_err(ConnectorError::new_schema_parser_error)?;
                Ok(sql_schema_calculator::calculate_sql_schema(
                    &schema,
                    self.flavour.as_ref(),
                ))
            }
            DiffTarget::Migrations(migrations) => self
                .flavour
                .sql_schema_from_migration_history(migrations)
                .await
                .map(From::from),
            DiffTarget::Database => self.flavour.describe_schema().await.map(From::from),
            DiffTarget::Empty => Ok(SqlDatabaseSchema::default()),
        }
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    fn set_host(&mut self, host: Arc<dyn migration_connector::ConnectorHost>) {
        self.host = host;
    }

    fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()> {
        self.flavour.set_params(params)
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
        SqlDatabaseSchema::default().into()
    }

    async fn ensure_connection_validity(&mut self) -> ConnectorResult<()> {
        self.flavour.ensure_connection_validity().await
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
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>> {
        Box::pin(async move { self.db_schema_from_diff_target(&diff_target).await.map(From::from) })
    }

    async fn db_execute(&mut self, script: String) -> ConnectorResult<()> {
        self.flavour.raw_cmd(&script).await
    }

    fn diff(&self, from: DatabaseSchema, to: DatabaseSchema) -> ConnectorResult<Migration> {
        let previous_schema = SqlDatabaseSchema::from_erased(from);
        let next_schema = SqlDatabaseSchema::from_erased(to);

        let steps =
            sql_schema_differ::calculate_steps(Pair::new(&previous_schema, &next_schema), self.flavour.as_ref());

        // let added_columns_with_virtual_defaults: Vec<(TableId, ColumnId)> =
        //     if let Some(next_datamodel) = to.as_datamodel() {
        //         let schema = datamodel::parse_schema_parserdb(next_datamodel)
        //             .map_err(ConnectorError::new_schema_parser_error)?;
        //         walk_added_columns(&steps)
        //             .map(|(table_index, column_index)| {
        //                 let table = next_schema.table_walker_at(table_index);
        //                 let column = table.column_at(column_index);

        //                 (table, column)
        //             })
        //             .filter(|(table, column)| {
        //                 schema
        //                     .db
        //                     .walk_models()
        //                     .find(|model| model.database_name() == table.name())
        //                     .and_then(|model| model.scalar_fields().find(|sf| sf.name() == column.name()))
        //                     .filter(|field| {
        //                         field
        //                             .default_value()
        //                             .map(|default| default.is_uuid() || default.is_cuid())
        //                             .unwrap_or(false)
        //                     })
        //                     .is_some()
        //             })
        //             .map(move |(table, column)| (table.table_id(), column.column_id()))
        //             .collect()
        //     } else {
        //         Vec::new()
        //     };

        Ok(Migration::new(SqlMigration {
            before: previous_schema.describer_schema,
            after: next_schema.describer_schema,
            steps,
        }))
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.flavour.drop_database()
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

    fn reset(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async {
            if self.flavour.reset().await.is_err() {
                best_effort_reset(self.flavour.as_mut()).await?;
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
    async fn validate_migrations(&mut self, migrations: &[MigrationDirectory]) -> ConnectorResult<()> {
        self.flavour.sql_schema_from_migration_history(migrations).await?;

        Ok(())
    }
}

enum ShadowDatabaseConfig<'a> {
    UserProvidedConnectionString(&'a str),
    GeneratedName(String),
}

impl ShadowDatabaseConfig<'_> {
    fn new_generated_name() -> ShadowDatabaseConfig<'static> {
        ShadowDatabaseConfig::GeneratedName(format!("prisma_migrate_shadow_db_{}", uuid::Uuid::new_v4()))
    }
}

impl<'a> From<&'a ConnectorParams> for ShadowDatabaseConfig<'a> {
    fn from(params: &'a ConnectorParams) -> Self {
        params
            .shadow_database_connection_string
            .as_ref()
            .map(|s| ShadowDatabaseConfig::UserProvidedConnectionString(s))
            .unwrap_or_else(|| ShadowDatabaseConfig::new_generated_name())
    }
}

/// Try to reset the database to an empty state. This should only be used
/// when we don't have the permissions to do a full reset.
#[tracing::instrument(skip(flavour))]
async fn best_effort_reset(flavour: &mut (dyn SqlFlavour + Send + Sync)) -> ConnectorResult<()> {
    best_effort_reset_impl(flavour)
        .await
        .map_err(|err| err.into_soft_reset_failed_error())
}

async fn best_effort_reset_impl(flavour: &mut (dyn SqlFlavour + Send + Sync)) -> ConnectorResult<()> {
    tracing::info!("Attempting best_effort_reset");

    let source_schema = flavour.describe_schema().await?;
    let target_schema = SqlSchema::default();
    let mut steps = Vec::new();

    // We drop views here, not in the normal migration process to not
    // accidentally drop something we can't describe in the data model.
    let drop_views = source_schema
        .view_walkers()
        .filter(|view| !flavour.view_should_be_ignored(view.name()))
        .map(|vw| vw.view_index())
        .map(DropView::new)
        .map(SqlMigrationStep::DropView);

    steps.extend(drop_views);

    let diffables: Pair<SqlDatabaseSchema> = Pair::new(source_schema, target_schema).map(From::from);
    steps.extend(sql_schema_differ::calculate_steps(diffables.as_ref(), flavour));
    let (source_schema, target_schema) = diffables.map(|s| s.describer_schema).into_tuple();

    let drop_udts = source_schema
        .udt_walkers()
        .map(|udtw| udtw.udt_index())
        .map(DropUserDefinedType::new)
        .map(SqlMigrationStep::DropUserDefinedType);

    steps.extend(drop_udts);

    let migration = SqlMigration {
        before: source_schema,
        after: target_schema,
        steps,
    };

    if migration.before.table_walker("_prisma_migrations").is_some() {
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
