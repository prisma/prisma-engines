//! Query Engine test setup.

#![allow(clippy::await_holding_lock)]
mod cockroachdb;
pub mod driver_adapters;
mod mongodb;
mod mssql;
mod mysql;
mod postgres;
mod providers;
mod sqlite;

pub use schema_core::schema_connector::ConnectorError;
use sqlite::sqlite_setup;

use self::{cockroachdb::*, mongodb::*, mssql::*, mysql::*, postgres::*};
use driver_adapters::DriverAdapter;
use enumflags2::BitFlags;
use providers::Provider;
use psl::{builtin_connectors::*, Datasource};
use schema_core::schema_connector::{ConnectorResult, DiffTarget, SchemaConnector};
use std::env;

pub trait ExternalInitializer<'a>
where
    Self: Sized,
{
    #[allow(async_fn_in_trait)]
    async fn init_with_migration(&self, script: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    #[allow(async_fn_in_trait)]
    async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn url(&self) -> &'a str;
    fn datamodel(&self) -> &'a str;
}

fn parse_configuration(datamodel: &str) -> ConnectorResult<(Datasource, String, BitFlags<psl::PreviewFeature>)> {
    let config = psl::parse_configuration(datamodel)
        .map_err(|err| ConnectorError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let url = config.datasources[0]
        .load_url(|key| env::var(key).ok())
        .map_err(|err| ConnectorError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let preview_features = config.preview_features();

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| ConnectorError::from_msg("There is no datasource in the schema.".into()))?;

    Ok((source, url, preview_features))
}

/// Database setup for connector-test-kit-rs with Driver Adapters.
/// If the external driver adapter requires a migration by means of the JavaScript runtime
/// (rather than just the Schema Engine), this function will call [`ExternalInitializer::init_with_migration`].
/// Otherwise, it will call [`ExternalInitializer::init`], and then proceed with the standard
/// setup based on the Schema Engine.
pub async fn setup_external<'a, EI>(
    driver_adapter: DriverAdapter,
    initializer: EI,
    db_schemas: &[&str],
) -> ConnectorResult<()>
where
    EI: ExternalInitializer<'a> + ?Sized,
{
    let prisma_schema = initializer.datamodel();
    let (source, url, _preview_features) = parse_configuration(prisma_schema)?;

    if driver_adapter == DriverAdapter::D1 {
        // 1. Compute the diff migration script.
        std::fs::remove_file(source.url.as_literal().unwrap().trim_start_matches("file:")).ok();
        let mut connector = sql_schema_connector::SqlSchemaConnector::new_sqlite();
        let migration_script = crate::diff(prisma_schema, url, &mut connector).await?;

        // 2. Tell JavaScript to take care of the schema migration.
        //    This results in a JSON-RPC call to the JS runtime.
        //    The JSON-RPC machinery is defined in the `[query-tests-setup]` crate, and it
        //    implements the `ExternalInitializer<'a>` trait.
        initializer
            .init_with_migration(migration_script)
            .await
            .map_err(|err| ConnectorError::from_msg(format!("Error migrating with D1 adapter: {}", err)))?;
    } else {
        setup(prisma_schema, db_schemas).await?;

        // 3. Tell JavaScript to initialize the external test session.
        //    The schema migration is taken care of by the Schema Engine.
        initializer.init().await.map_err(|err| {
            ConnectorError::from_msg(format!("Error initializing {} adapter: {}", driver_adapter, err))
        })?;
    }

    Ok(())
}

/// Database setup for connector-test-kit-rs.
pub async fn setup(prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let (source, url, _preview_features) = parse_configuration(prisma_schema)?;

    let provider = Provider::try_from(source.active_provider).ok();

    match provider {
        Some(Provider::SqlServer) => mssql_setup(url, prisma_schema, db_schemas).await,
        Some(Provider::Postgres) => postgres_setup(url, prisma_schema, db_schemas).await,
        Some(Provider::Cockroach) => cockroach_setup(url, prisma_schema).await,
        Some(Provider::Mysql) => mysql_setup(url, prisma_schema).await,
        Some(Provider::Mongo) => mongo_setup(prisma_schema, &url).await,
        Some(Provider::Sqlite) => sqlite_setup(source, url, prisma_schema).await,
        None => unimplemented!("Connector is not supported yet"),
    }
}

/// Database teardown for connector-test-kit-rs.
pub async fn teardown(prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let (source, url, _) = parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider if [POSTGRES.provider_name()].contains(provider) => {
            postgres_teardown(&url, db_schemas).await?;
        }

        provider
            if [
                SQLITE.provider_name(),
                MSSQL.provider_name(),
                MYSQL.provider_name(),
                MONGODB.provider_name(),
                COCKROACH.provider_name(),
            ]
            .contains(provider) => {}

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}

/// Compute an initialisation migration script via
/// `prisma migrate diff --from-empty --to-schema-datamodel $SCHEMA_PATH --script`.
pub(crate) async fn diff(schema: &str, url: String, connector: &mut dyn SchemaConnector) -> ConnectorResult<String> {
    connector.set_params(schema_core::schema_connector::ConnectorParams {
        connection_string: url,
        preview_features: Default::default(),
        shadow_database_connection_string: None,
    })?;
    let from = connector
        .database_schema_from_diff_target(DiffTarget::Empty, None, None)
        .await?;
    let to = connector
        .database_schema_from_diff_target(
            DiffTarget::Datamodel(vec![("schema.prisma".to_string(), schema.into())]),
            None,
            None,
        )
        .await?;
    let migration = connector.diff(from, to);
    connector.render_script(&migration, &Default::default())
}

/// Apply the script returned by [`diff`] against the database.
pub(crate) async fn diff_and_apply(
    schema: &str,
    url: String,
    connector: &mut dyn SchemaConnector,
) -> ConnectorResult<()> {
    let script = diff(schema, url, connector).await.unwrap();
    connector.db_execute(script).await
}
