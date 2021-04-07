//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::trivial_regex)] // these will grow

mod connection_wrapper;
mod error;
mod flavour;
mod pair;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_change_checker;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

use connection_wrapper::Connection;
use datamodel::Datamodel;
use enumflags2::BitFlags;
use error::quaint_error_to_connector_error;
use flavour::SqlFlavour;
use migration_connector::*;
use pair::Pair;
use quaint::{prelude::ConnectionInfo, single::Quaint};
use sql_migration::{DropView, SqlMigration, SqlMigrationStep};
use sql_schema_describer::{walkers::SqlSchemaExt, SqlSchema};
use user_facing_errors::{common::InvalidDatabaseString, KnownError};

/// The top-level SQL migration connector.
pub struct SqlMigrationConnector {
    connection: Connection,
    flavour: Box<dyn SqlFlavour + Send + Sync + 'static>,
    shadow_database_connection_string: Option<String>,
}

impl SqlMigrationConnector {
    /// Construct and initialize the SQL migration connector.
    pub async fn new(
        connection_string: &str,
        features: BitFlags<MigrationFeature>,
        shadow_database_connection_string: Option<String>,
    ) -> ConnectorResult<Self> {
        let connection = connect(connection_string).await?;
        let flavour = flavour::from_connection_info(connection.connection_info(), features);

        flavour.ensure_connection_validity(&connection).await?;

        Ok(Self {
            flavour,
            connection,
            shadow_database_connection_string,
        })
    }

    /// Create the database corresponding to the connection string, without initializing the connector.
    pub async fn create_database(database_str: &str) -> ConnectorResult<String> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let flavour = flavour::from_connection_info(&connection_info, BitFlags::empty());
        flavour.create_database(database_str).await
    }

    /// Drop the database corresponding to the connection string, without initializing the connector.
    pub async fn drop_database(database_str: &str) -> ConnectorResult<()> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let flavour = flavour::from_connection_info(&connection_info, BitFlags::empty());

        flavour.drop_database(database_str).await
    }

    /// Set up the database for connector-test-kit, without initializing the connector.
    pub async fn qe_setup(database_str: &str) -> ConnectorResult<()> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        let flavour = flavour::from_connection_info(&connection_info, BitFlags::empty());

        flavour.qe_setup(database_str).await
    }

    fn conn(&self) -> &Connection {
        &self.connection
    }

    fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync) {
        self.flavour.as_ref()
    }

    /// Made public for tests.
    pub fn quaint(&self) -> &Quaint {
        self.connection.quaint()
    }

    /// Made public for tests.
    pub async fn describe_schema(&self) -> ConnectorResult<SqlSchema> {
        self.flavour.describe_schema(&self.connection).await
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

        let source_schema = self.flavour.describe_schema(connection).await?;
        let target_schema = SqlSchema::empty();

        let mut steps = Vec::new();

        // We drop views here, not in the normal migration process to not
        // accidentally drop something we can't describe in the data model.
        let drop_views = source_schema
            .view_walkers()
            .map(|vw| vw.view_index())
            .map(DropView::new)
            .map(SqlMigrationStep::DropView);

        steps.extend(drop_views);

        steps.extend(sql_schema_differ::calculate_steps(
            Pair::new(&source_schema, &target_schema),
            self.flavour.as_ref(),
        ));

        let migration = SqlMigration {
            added_columns_with_virtual_defaults: Vec::new(),
            before: source_schema,
            after: target_schema,
            steps,
        };

        self.apply_migration(&migration).await?;

        if migration.before.table_walker("_prisma_migrations").is_some() {
            self.flavour.drop_migrations_table(self.conn()).await?;
        }

        Ok(())
    }

    /// Generate a name for a temporary (shadow) database, _if_ there is no user-configured shadow database url.
    fn shadow_database_name(&self) -> Option<String> {
        if self.shadow_database_connection_string.is_some() {
            return None;
        }

        Some(format!("prisma_migrate_shadow_db_{}", uuid::Uuid::new_v4()))
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn connector_type(&self) -> &'static str {
        self.connection.connection_info().sql_family().as_str()
    }

    async fn acquire_lock(&self) -> ConnectorResult<()> {
        self.flavour().acquire_lock(self.conn()).await
    }

    async fn version(&self) -> ConnectorResult<String> {
        Ok(self
            .connection
            .version()
            .await?
            .unwrap_or_else(|| "Database version information not available.".into()))
    }

    async fn create_database(database_str: &str) -> ConnectorResult<String> {
        Self::create_database(database_str).await
    }

    async fn reset(&self) -> ConnectorResult<()> {
        if self.flavour.reset(self.conn()).await.is_err() {
            self.best_effort_reset(self.conn()).await?;
        }

        Ok(())
    }

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        datamodel: &Datamodel,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        self.flavour.check_database_version_compatibility(datamodel)
    }

    fn database_migration_inferrer(&self) -> &dyn DatabaseMigrationInferrer<SqlMigration> {
        self
    }

    fn database_migration_step_applier(&self) -> &dyn DatabaseMigrationStepApplier<SqlMigration> {
        self
    }

    fn destructive_change_checker(&self) -> &dyn DestructiveChangeChecker<SqlMigration> {
        self
    }

    fn migration_persistence(&self) -> &dyn MigrationPersistence {
        self
    }
}

async fn connect(database_str: &str) -> ConnectorResult<Connection> {
    let connection_info = ConnectionInfo::from_url(database_str).map_err(|err| {
        let details = user_facing_errors::quaint::invalid_url_description(database_str, &err.to_string());
        KnownError::new(InvalidDatabaseString { details })
    })?;

    let connection = Quaint::new(database_str)
        .await
        .map_err(|err| quaint_error_to_connector_error(err, &connection_info))?;

    Ok(Connection::new(connection))
}
