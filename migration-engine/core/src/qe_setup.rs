//! Query Engine test setup.

use crate::api::GenericApi;
use crate::commands::SchemaPushInput;
use crate::core_error::{CoreError, CoreResult};
#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
use datamodel::common::provider_names::{
    MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME,
};
use enumflags2::BitFlags;
#[cfg(feature = "mongodb")]
use mongodb_migration_connector::MongoDbMigrationConnector;
use sql_migration_connector::SqlMigrationConnector;

/// Flags from Query Engine to define the underlying database features.
#[derive(Debug, Clone, Copy, BitFlags)]
#[repr(u8)]
pub enum QueryEngineFlags {
    /// We cannot `CREATE` (or `DROP`) databases.
    DatabaseCreationNotAllowed = 0x01,
}

/// Database setup for connector-test-kit.
pub async fn run(prisma_schema: &str, flags: BitFlags<QueryEngineFlags>) -> CoreResult<()> {
    let config = super::parse_configuration(prisma_schema)?;
    let features = super::features::from_config(&config);

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    let api: Box<dyn GenericApi> = match &source.active_provider {
        _ if flags.contains(QueryEngineFlags::DatabaseCreationNotAllowed) => {
            let api = SqlMigrationConnector::new(&source.url().value, features, None).await?;
            api.reset().await?;

            Box::new(api)
        }
        provider
            if [
                MYSQL_SOURCE_NAME,
                POSTGRES_SOURCE_NAME,
                SQLITE_SOURCE_NAME,
                MSSQL_SOURCE_NAME,
            ]
            .contains(&provider.as_str()) =>
        {
            // 1. creates schema & database
            SqlMigrationConnector::qe_setup(&source.url().value).await?;
            Box::new(SqlMigrationConnector::new(&source.url().value, features, None).await?)
        }
        #[cfg(feature = "mongodb")]
        provider if provider == MONGODB_SOURCE_NAME => {
            MongoDbMigrationConnector::qe_setup(&source.url().value).await?;
            let connector = MongoDbMigrationConnector::new(&source.url().value, features).await?;
            Box::new(connector)
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    };

    // 2. create the database schema for given Prisma schema
    let schema_push_input = SchemaPushInput {
        schema: prisma_schema.to_string(),
        assume_empty: true,
        force: true,
    };

    api.schema_push(&schema_push_input).await?;
    Ok(())
}
