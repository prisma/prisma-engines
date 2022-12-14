//! The MongoDB migration connector.
//!
//! It is intentionally structured after sql-migration-connector and implements the same
//! [MigrationConnector](/trait.MigrationConnector.html) API.

mod client_wrapper;
mod destructive_change_checker;
mod differ;
mod migration;
mod migration_persistence;
mod migration_step_applier;
mod schema_calculator;

use client_wrapper::Client;
use enumflags2::BitFlags;
use migration::MongoDbMigration;
use migration_connector::{migrations_directory::MigrationDirectory, *};
use mongodb_schema_describer::MongoSchema;
use psl::PreviewFeature;
use std::{future, sync::Arc};
use tokio::sync::OnceCell;

/// The top-level MongoDB migration connector.
pub struct MongoDbMigrationConnector {
    connection_string: String,
    client: OnceCell<Client>,
    preview_features: BitFlags<PreviewFeature>,
    host: Arc<dyn ConnectorHost>,
}

impl MongoDbMigrationConnector {
    pub fn new(params: ConnectorParams) -> Self {
        Self {
            connection_string: params.connection_string,
            preview_features: params.preview_features,
            client: OnceCell::new(),
            host: Arc::new(EmptyHost),
        }
    }

    async fn client(&self) -> ConnectorResult<&Client> {
        let client: &Client = self
            .client
            .get_or_try_init(move || {
                Box::pin(async move { Client::connect(&self.connection_string, self.preview_features).await })
            })
            .await?;

        Ok(client)
    }

    async fn mongodb_schema_from_diff_target(&self, target: DiffTarget<'_>) -> ConnectorResult<MongoSchema> {
        match target {
            DiffTarget::Datamodel(schema) => {
                let validated_schema = psl::parse_schema(schema).map_err(ConnectorError::new_schema_parser_error)?;
                Ok(schema_calculator::calculate(&validated_schema))
            }
            DiffTarget::Database => self.client().await?.describe().await,
            DiffTarget::Migrations(_) => Err(unsupported_command_error()),
            DiffTarget::Empty => Ok(MongoSchema::default()),
        }
    }
}

impl MigrationConnector for MongoDbMigrationConnector {
    fn connection_string(&self) -> Option<&str> {
        Some(&self.connection_string)
    }

    fn database_schema_from_diff_target<'a>(
        &'a mut self,
        diff_target: DiffTarget<'a>,
        _shadow_database_connection_string: Option<String>,
        _namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>> {
        Box::pin(async {
            let schema = self.mongodb_schema_from_diff_target(diff_target).await?;
            Ok(DatabaseSchema::new(schema))
        })
    }

    fn host(&self) -> &Arc<dyn ConnectorHost> {
        &self.host
    }

    fn apply_migration<'a>(&'a mut self, migration: &'a Migration) -> BoxFuture<'a, ConnectorResult<u32>> {
        Box::pin(self.apply_migration_impl(migration))
    }

    fn apply_script(&mut self, _migration_name: &str, _script: &str) -> BoxFuture<ConnectorResult<()>> {
        Box::pin(future::ready(Err(crate::unsupported_command_error())))
    }

    fn connector_type(&self) -> &'static str {
        "mongodb"
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            let name = self.client().await?.db_name();
            tracing::warn!("MongoDB database will be created on first use.");
            Ok(name.into())
        })
    }

    fn db_execute(&mut self, _script: String) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(future::ready(Err(ConnectorError::from_msg(
            "dbExecute is not supported on MongoDB".to_owned(),
        ))))
    }

    fn empty_database_schema(&self) -> DatabaseSchema {
        DatabaseSchema::new(MongoSchema::default())
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(future::ready(Ok(())))
    }

    fn version(&mut self) -> BoxFuture<'_, migration_connector::ConnectorResult<String>> {
        Box::pin(future::ready(Ok("4 or 5".to_owned())))
    }

    fn diff(&self, from: DatabaseSchema, to: DatabaseSchema) -> Migration {
        let from: Box<MongoSchema> = from.downcast();
        let to: Box<MongoSchema> = to.downcast();
        Migration::new(differ::diff(from, to))
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async { self.client().await?.drop_database().await })
    }

    fn migration_file_extension(&self) -> &'static str {
        unreachable!("migration_file_extension")
    }

    fn migration_len(&self, migration: &Migration) -> usize {
        migration.downcast_ref::<MongoDbMigration>().steps.len()
    }

    fn migration_summary(&self, migration: &Migration) -> String {
        migration.downcast_ref::<MongoDbMigration>().summary()
    }

    fn reset(
        &mut self,
        _soft: bool,
        _namespaces: Option<Namespaces>,
    ) -> BoxFuture<'_, migration_connector::ConnectorResult<()>> {
        Box::pin(async { self.client().await?.drop_database().await })
    }

    fn migration_persistence(&mut self) -> &mut dyn migration_connector::MigrationPersistence {
        self
    }

    fn destructive_change_checker(&mut self) -> &mut dyn migration_connector::DestructiveChangeChecker {
        self
    }

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(future::ready(Ok(())))
    }

    fn introspect<'a>(
        &'a mut self,
        ctx: &'a IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<IntrospectionResult>> {
        Box::pin(async move {
            let url: String = ctx.datasource().load_url(|v| std::env::var(v).ok()).map_err(|err| {
                migration_connector::ConnectorError::new_schema_parser_error(
                    err.to_pretty_string("schema.prisma", ctx.schema_string()),
                )
            })?;
            let connector = mongodb_introspection_connector::MongoDbIntrospectionConnector::new(&url)
                .await
                .map_err(|err| ConnectorError::from_source(err, "Introspection error"))?;
            connector
                .introspect(ctx)
                .await
                .map_err(|err| ConnectorError::from_source(err, "Introspection error"))
        })
    }

    fn render_script(
        &self,
        _migration: &Migration,
        _diagnostics: &DestructiveChangeDiagnostics,
    ) -> ConnectorResult<String> {
        Err(ConnectorError::from_msg(
            "Rendering to a script is not supported on MongoDB.".to_owned(),
        ))
    }

    fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()> {
        self.connection_string = params.connection_string;
        self.preview_features = params.preview_features;
        Ok(())
    }

    fn set_preview_features(&mut self, preview_features: BitFlags<psl::PreviewFeature>) {
        self.preview_features = preview_features;
    }

    fn set_host(&mut self, host: Arc<dyn migration_connector::ConnectorHost>) {
        self.host = host;
    }

    fn validate_migrations<'a>(
        &'a mut self,
        _migrations: &'a [MigrationDirectory],
        _namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(future::ready(Ok(())))
    }

    fn extract_namespaces(&self, _schema: &DatabaseSchema) -> Option<Namespaces> {
        None
    }
}

fn unsupported_command_error() -> ConnectorError {
    ConnectorError::from_msg(
"The \"mongodb\" provider is not supported with this command. For more info see https://www.prisma.io/docs/concepts/database-connectors/mongodb".to_owned()

        )
}
