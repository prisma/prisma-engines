//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::trivial_regex)] // these will grow

mod connection_wrapper;
mod error;
mod flavour;
mod pair;
mod sql_database_step_applier;
mod sql_destructive_change_checker;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

use connection_wrapper::{connect, Connection};
use datamodel::{common::preview_features::PreviewFeature, walkers::walk_models, Configuration, Datamodel};
use enumflags2::BitFlags;
use flavour::SqlFlavour;
use migration_connector::{migrations_directory::MigrationDirectory, *};
use pair::Pair;
use quaint::prelude::ConnectionInfo;
use sql_migration::{DropUserDefinedType, DropView, SqlMigration, SqlMigrationStep};
use sql_schema_describer::{walkers::SqlSchemaExt, ColumnId, SqlSchema, TableId};
use std::env;
use user_facing_errors::KnownError;

/// The top-level SQL migration connector.
pub struct SqlMigrationConnector {
    connection: tokio::sync::OnceCell<ConnectorResult<Connection>>,
    connection_string: String,
    connection_info: ConnectionInfo,
    flavour: Box<dyn SqlFlavour + Send + Sync + 'static>,
    shadow_database_connection_string: Option<String>,
}

impl SqlMigrationConnector {
    /// Construct and initialize the SQL migration connector.
    pub fn new(
        connection_string: String,
        preview_features: BitFlags<PreviewFeature>,
        shadow_database_connection_string: Option<String>,
    ) -> ConnectorResult<Self> {
        let connection_info = ConnectionInfo::from_url(&connection_string).map_err(|err| {
            let details = user_facing_errors::quaint::invalid_connection_string_description(&err.to_string());
            KnownError::new(user_facing_errors::common::InvalidConnectionString { details })
        })?;

        let flavour = flavour::from_connection_info(&connection_info, preview_features);

        Ok(Self {
            connection_string,
            connection_info,
            connection: tokio::sync::OnceCell::new(),
            flavour,
            shadow_database_connection_string,
        })
    }

    /// Set up the database for connector-test-kit, without initializing the connector.
    pub async fn qe_setup(database_str: &str) -> ConnectorResult<()> {
        let connection_info = ConnectionInfo::from_url(database_str).map_err(ConnectorError::url_parse_error)?;

        let flavour = flavour::from_connection_info(&connection_info, BitFlags::empty());

        flavour.qe_setup(database_str).await
    }

    async fn conn(&self) -> ConnectorResult<&Connection> {
        self.connection
            .get_or_init(|| {
                Box::pin(async {
                    let connection = connect(&self.connection_string).await?;
                    self.flavour.ensure_connection_validity(&connection).await?;
                    Ok(connection)
                })
            })
            .await
            .as_ref()
            .map_err(|err| err.clone())
    }

    fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync) {
        self.flavour.as_ref()
    }

    /// Made public for tests.
    pub async fn describe_schema(&self) -> ConnectorResult<SqlSchema> {
        self.conn().await?.describe_schema().await
    }

    /// Try to reset the database to an empty state. This should only be used
    /// when we don't have the permissions to do a full reset.
    #[tracing::instrument(skip(self))]
    async fn best_effort_reset(&self, connection: &Connection) -> ConnectorResult<()> {
        self.best_effort_reset_impl(connection)
            .await
            .map_err(|err| err.into_soft_reset_failed_error())
    }

    async fn best_effort_reset_impl(&self, connection: &Connection) -> ConnectorResult<()> {
        tracing::info!("Attempting best_effort_reset");

        let source_schema = connection.describe_schema().await?;
        let target_schema = SqlSchema::empty();
        let mut steps = Vec::new();

        // We drop views here, not in the normal migration process to not
        // accidentally drop something we can't describe in the data model.
        let drop_views = source_schema
            .view_walkers()
            .filter(|view| !self.flavour.view_should_be_ignored(view.name()))
            .map(|vw| vw.view_index())
            .map(DropView::new)
            .map(SqlMigrationStep::DropView);

        steps.extend(drop_views);

        steps.extend(sql_schema_differ::calculate_steps(
            Pair::new(&source_schema, &target_schema),
            self.flavour.as_ref(),
        ));

        let drop_udts = source_schema
            .udt_walkers()
            .map(|udtw| udtw.udt_index())
            .map(DropUserDefinedType::new)
            .map(SqlMigrationStep::DropUserDefinedType);

        steps.extend(drop_udts);

        let migration = SqlMigration {
            added_columns_with_virtual_defaults: Vec::new(),
            before: source_schema,
            after: target_schema,
            steps,
        };

        if migration.before.table_walker("_prisma_migrations").is_some() {
            self.flavour.drop_migrations_table(connection).await?;
        }

        if migration.steps.is_empty() {
            return Ok(());
        }

        let migration = self.render_script(&Migration::new(migration), &DestructiveChangeDiagnostics::default());
        connection.raw_cmd(&migration).await?;

        Ok(())
    }

    /// For tests.
    pub fn migration_from_schemas(
        from: (&Configuration, &Datamodel),
        to: (&Configuration, &Datamodel),
    ) -> SqlMigration {
        let connection_info =
            ConnectionInfo::from_url(&from.0.datasources[0].load_url(|key| env::var(key).ok()).unwrap()).unwrap();

        let flavour = flavour::from_connection_info(&connection_info, BitFlags::empty());
        let from_sql = sql_schema_calculator::calculate_sql_schema(from, flavour.as_ref());
        let to_sql = sql_schema_calculator::calculate_sql_schema(to, flavour.as_ref());

        let steps = sql_schema_differ::calculate_steps(Pair::new(&from_sql, &to_sql), flavour.as_ref());

        SqlMigration {
            before: from_sql,
            after: to_sql,
            added_columns_with_virtual_defaults: Vec::new(),
            steps,
        }
    }

    /// For tests
    pub async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        let conn = self.conn().await?;
        Ok(conn.query_raw(sql, params).await?)
    }

    /// For tests
    pub async fn query(&self, query: impl Into<quaint::ast::Query<'_>>) -> ConnectorResult<quaint::prelude::ResultSet> {
        let conn = self.conn().await?;
        Ok(conn.query(query).await?)
    }

    /// For tests
    pub async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        let conn = self.conn().await?;
        Ok(conn.raw_cmd(sql).await?)
    }

    /// Generate a name for a temporary (shadow) database, _if_ there is no user-configured shadow database url.
    fn shadow_database_name(&self) -> Option<String> {
        if self.shadow_database_connection_string.is_some() {
            return None;
        }

        Some(format!("prisma_migrate_shadow_db_{}", uuid::Uuid::new_v4()))
    }

    async fn sql_schema_from_diff_target(&self, target: &DiffTarget<'_>) -> ConnectorResult<SqlSchema> {
        match target {
            DiffTarget::Datamodel(schema) => Ok(sql_schema_calculator::calculate_sql_schema(
                (schema.0, schema.1),
                self.flavour.as_ref(),
            )),
            DiffTarget::Migrations(migrations) => {
                let conn = self.conn().await?;
                self.flavour()
                    .sql_schema_from_migration_history(migrations, conn, self)
                    .await
            }
            DiffTarget::Database => self.describe_schema().await,
            DiffTarget::Empty => Ok(SqlSchema::empty()),
        }
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    fn connector_type(&self) -> &'static str {
        self.connection_info.sql_family().as_str()
    }

    async fn acquire_lock(&self) -> ConnectorResult<()> {
        let conn = self.conn().await?;
        self.flavour().acquire_lock(conn).await
    }

    async fn ensure_connection_validity(&self) -> ConnectorResult<()> {
        let conn = self.conn().await?;
        self.flavour().ensure_connection_validity(conn).await
    }

    async fn version(&self) -> ConnectorResult<String> {
        let conn = self.conn().await?;
        Ok(conn
            .version()
            .await?
            .unwrap_or_else(|| "Database version information not available.".into()))
    }

    async fn create_database(&self) -> ConnectorResult<String> {
        self.flavour.create_database(&self.connection_string).await
    }

    async fn diff(&self, from: DiffTarget<'_>, to: DiffTarget<'_>) -> ConnectorResult<Migration> {
        let previous_schema = self.sql_schema_from_diff_target(&from).await?;
        let next_schema = self.sql_schema_from_diff_target(&to).await?;

        let steps =
            sql_schema_differ::calculate_steps(Pair::new(&previous_schema, &next_schema), self.flavour.as_ref());

        let added_columns_with_virtual_defaults: Vec<(TableId, ColumnId)> =
            if let Some((_, next_datamodel)) = to.as_datamodel() {
                walk_added_columns(&steps)
                    .map(|(table_index, column_index)| {
                        let table = next_schema.table_walker_at(table_index);
                        let column = table.column_at(column_index);

                        (table, column)
                    })
                    .filter(|(table, column)| {
                        walk_models(next_datamodel)
                            .find(|model| model.database_name() == table.name())
                            .and_then(|model| model.find_scalar_field(column.name()))
                            .filter(|field| {
                                field
                                    .default_value()
                                    .map(|default| default.is_uuid() || default.is_cuid())
                                    .unwrap_or(false)
                            })
                            .is_some()
                    })
                    .map(move |(table, column)| (table.table_id(), column.column_id()))
                    .collect()
            } else {
                Vec::new()
            };

        Ok(Migration::new(SqlMigration {
            before: previous_schema,
            after: next_schema,
            added_columns_with_virtual_defaults,
            steps,
        }))
    }

    async fn drop_database(&self) -> ConnectorResult<()> {
        self.flavour.drop_database(&self.connection_string).await
    }

    fn migration_file_extension(&self) -> &'static str {
        "sql"
    }

    fn migration_len(&self, migration: &Migration) -> usize {
        migration.downcast_ref::<SqlMigration>().steps.len()
    }

    async fn reset(&self) -> ConnectorResult<()> {
        let conn = self.conn().await?;
        if self.flavour.reset(conn).await.is_err() {
            self.best_effort_reset(conn).await?;
        }

        Ok(())
    }

    fn migration_summary(&self, migration: &Migration) -> String {
        migration.downcast_ref::<SqlMigration>().drift_summary()
    }

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        datamodel: &Datamodel,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        self.flavour.check_database_version_compatibility(datamodel)
    }

    fn database_migration_step_applier(&self) -> &dyn DatabaseMigrationStepApplier {
        self
    }

    fn destructive_change_checker(&self) -> &dyn DestructiveChangeChecker {
        self
    }

    fn migration_persistence(&self) -> &dyn MigrationPersistence {
        self
    }

    #[tracing::instrument(skip(self, migrations))]
    async fn validate_migrations(&self, migrations: &[MigrationDirectory]) -> ConnectorResult<()> {
        let conn = self.conn().await?;
        self.flavour()
            .sql_schema_from_migration_history(migrations, conn, self)
            .await?;

        Ok(())
    }
}

/// List all the columns added in the migration, either by alter table steps or
/// redefine table steps.
///
/// The return value should be interpreted as an iterator over `(table_index,
/// column_index)` in the `next` schema.
fn walk_added_columns(steps: &[SqlMigrationStep]) -> impl Iterator<Item = (TableId, ColumnId)> + '_ {
    steps
        .iter()
        .filter_map(|step| step.as_alter_table())
        .flat_map(move |alter_table| {
            alter_table
                .changes
                .iter()
                .filter_map(|change| change.as_add_column())
                .map(move |column_index| -> (TableId, ColumnId) { (*alter_table.table_ids.next(), column_index) })
        })
        .chain(
            steps
                .iter()
                .filter_map(|step| step.as_redefine_tables())
                .flatten()
                .flat_map(move |table| {
                    table
                        .added_columns
                        .iter()
                        .map(move |column_index| (*table.table_ids.next(), *column_index))
                }),
        )
}
