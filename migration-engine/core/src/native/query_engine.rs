//! Query Engine test setup.

#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
use datamodel::common::provider_names::{
    MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME,
};
use migration_connector::{ConnectorResult, DiffTarget, MigrationConnector};
#[cfg(feature = "mongodb")]
use mongodb_migration_connector::MongoDbMigrationConnector;
use sql_migration_connector::SqlMigrationConnector;

/// Database setup for connector-test-kit-rs.
pub async fn setup(prisma_schema: &str) -> ConnectorResult<()> {
    let (source, url, preview_features, _shadow_database_url) = super::parse_configuration(prisma_schema)?;

    match &source.active_provider {
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
            let api = SqlMigrationConnector::new(url, preview_features, None)?;

            // 2. create the database schema for given Prisma schema
            {
                let (config, schema) = crate::parse_schema(prisma_schema).unwrap();
                let migration = api
                    .diff(DiffTarget::Empty, DiffTarget::Datamodel((&config, &schema)))
                    .await
                    .unwrap();
                api.database_migration_step_applier()
                    .apply_migration(&migration)
                    .await
                    .unwrap();
            };
        }

        #[cfg(feature = "mongodb")]
        provider if provider == MONGODB_SOURCE_NAME => {
            let connector = MongoDbMigrationConnector::new(url, preview_features);
            // Drop database. Creation is automatically done when collections are created.
            connector.drop_database().await?;
            let (_, schema) = crate::parse_schema(prisma_schema).unwrap();
            connector.create_collections(&schema).await?;
        }

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}

/// Database teardown for connector-test-kit-rs.
pub async fn teardown(prisma_schema: &str) -> ConnectorResult<()> {
    let (source, url, _, _) = super::parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider
            if [
                MYSQL_SOURCE_NAME,
                POSTGRES_SOURCE_NAME,
                SQLITE_SOURCE_NAME,
                MSSQL_SOURCE_NAME,
            ]
            .contains(&provider.as_str()) =>
        {
            SqlMigrationConnector::qe_teardown(&url).await?;
        }

        #[cfg(feature = "mongodb")]
        provider if provider == MONGODB_SOURCE_NAME => {}

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}
