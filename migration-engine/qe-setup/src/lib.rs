//! Query Engine test setup.

mod mongodb;
mod mssql;
mod mysql;
mod postgres;

pub use migration_core::migration_connector::ConnectorError;

use self::{mongodb::*, mssql::*, mysql::*, postgres::*};
use datamodel::{
    common::{
        preview_features::*,
        provider_names::{
            COCKROACHDB_SOURCE_NAME, MONGODB_SOURCE_NAME, MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME,
            SQLITE_SOURCE_NAME,
        },
    },
    Datasource,
};
use enumflags2::BitFlags;
use migration_core::migration_connector::{ConnectorResult, DiffTarget};
use std::env;

fn parse_configuration(datamodel: &str) -> ConnectorResult<(Datasource, String, BitFlags<PreviewFeature>)> {
    let config = datamodel::parse_configuration(datamodel)
        .map(|validated_config| validated_config.subject)
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

/// Database setup for connector-test-kit-rs.
pub async fn setup(prisma_schema: &str) -> ConnectorResult<()> {
    let (source, url, _preview_features) = parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider if [POSTGRES_SOURCE_NAME, COCKROACHDB_SOURCE_NAME].contains(&provider.as_str()) => {
            postgres_setup(url, prisma_schema).await?
        }
        provider if [MSSQL_SOURCE_NAME].contains(&provider.as_str()) => mssql_setup(url, prisma_schema).await?,
        provider if [MYSQL_SOURCE_NAME].contains(&provider.as_str()) => {
            mysql_reset(&url).await?;
            let api = migration_core::migration_api(prisma_schema)?;
            let api = api.connector();
            let ast = datamodel::parse_schema_ast(prisma_schema).unwrap();
            let schema = datamodel::parse_schema_parserdb(prisma_schema, &ast).unwrap();
            let migration = api
                .diff(DiffTarget::Empty, DiffTarget::Datamodel(&schema))
                .await
                .unwrap();
            api.database_migration_step_applier()
                .apply_migration(&migration)
                .await
                .unwrap();
        }
        provider if [SQLITE_SOURCE_NAME].contains(&provider.as_str()) => {
            // 1. creates schema & database
            let api = migration_core::migration_api(prisma_schema)?;
            let api = api.connector();
            api.drop_database().await.ok();
            api.create_database().await?;

            // 2. create the database schema for given Prisma schema
            {
                let ast = datamodel::parse_schema_ast(prisma_schema).unwrap();
                let schema = datamodel::parse_schema_parserdb(prisma_schema, &ast).unwrap();
                let migration = api
                    .diff(DiffTarget::Empty, DiffTarget::Datamodel(&schema))
                    .await
                    .unwrap();
                api.database_migration_step_applier()
                    .apply_migration(&migration)
                    .await
                    .unwrap();
            };
        }

        provider if provider == MONGODB_SOURCE_NAME => mongo_setup(prisma_schema, &url).await?,

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}

/// Database teardown for connector-test-kit-rs.
pub async fn teardown(prisma_schema: &str) -> ConnectorResult<()> {
    let (source, url, _) = parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider if [POSTGRES_SOURCE_NAME, COCKROACHDB_SOURCE_NAME].contains(&provider.as_str()) => {
            postgres_teardown(&url).await?;
        }

        provider
            if [
                SQLITE_SOURCE_NAME,
                MSSQL_SOURCE_NAME,
                MYSQL_SOURCE_NAME,
                MONGODB_SOURCE_NAME,
            ]
            .contains(&provider.as_str()) => {}

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}
