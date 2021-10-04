//! Query Engine test setup.

use crate::{api::GenericApi, commands::SchemaPushInput, core_error::CoreResult};
#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
use datamodel::common::provider_names::{
    MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME,
};
#[cfg(feature = "mongodb")]
use mongodb_migration_connector::MongoDbMigrationConnector;
use sql_migration_connector::SqlMigrationConnector;

/// Database setup for connector-test-kit.
pub async fn run(prisma_schema: &str) -> CoreResult<()> {
    let (source, url, preview_features, _shadow_database_url) = super::parse_configuration(prisma_schema)?;

    let api: Box<dyn GenericApi> = match &source.active_provider {
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
            SqlMigrationConnector::qe_setup(&url).await?;
            Box::new(SqlMigrationConnector::new(url, preview_features, None)?)
        }
        #[cfg(feature = "mongodb")]
        provider if provider == MONGODB_SOURCE_NAME => {
            MongoDbMigrationConnector::qe_setup(&url).await?;
            let connector = MongoDbMigrationConnector::new(&url).await?;
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
