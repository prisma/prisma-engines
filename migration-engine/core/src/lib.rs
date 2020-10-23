#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! The top-level library crate for the migration engine.

#[allow(missing_docs)]
pub mod api;
pub mod commands;
#[allow(missing_docs)]
pub mod migration;
#[allow(missing_docs)]
pub mod migration_engine;

mod core_error;
mod gate_keeper;

use anyhow::anyhow;
pub use api::GenericApi;
pub use commands::{ApplyMigrationInput, InferMigrationStepsInput, MigrationStepsResultOutput, SchemaPushInput};
pub use core_error::{CoreError, CoreResult};
pub use gate_keeper::GateKeeper;

use commands::{MigrationCommand, SchemaPushCommand};
use datamodel::{
    common::provider_names::{MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    dml::Datamodel,
    Configuration,
};
use migration_connector::ConnectorError;
use migration_engine::MigrationEngine;
use sql_migration_connector::SqlMigrationConnector;
use std::sync::Arc;

/// Top-level constructor for the migration engine API.
pub async fn migration_api(
    datamodel: &str,
    enabled_preview_features: Vec<String>,
) -> CoreResult<Arc<dyn api::GenericApi>> {
    let config = parse_configuration(datamodel)?;

    GateKeeper::new(enabled_preview_features).any_blocked(config.preview_features())?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::Generic(anyhow!("There is no datasource in the schema.")))?;

    let connector = match &source.active_provider {
        #[cfg(feature = "sql")]
        provider if POSTGRES_SOURCE_NAME == provider => {
            let mut u = url::Url::parse(&source.url().value).map_err(|url_error| {
                CoreError::ConnectorError(ConnectorError::url_parse_error(url_error, &source.url().value))
            })?;

            let params: Vec<(String, String)> = u.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

            u.query_pairs_mut().clear();

            for (k, v) in params.into_iter() {
                if k == "statement_cache_size" {
                    u.query_pairs_mut().append_pair("statement_cache_size", "0");
                } else {
                    u.query_pairs_mut().append_pair(&k, &v);
                }
            }

            if !u.query_pairs().any(|(k, _)| k == "statement_cache_size") {
                u.query_pairs_mut().append_pair("statement_cache_size", "0");
            }

            SqlMigrationConnector::new(u.as_str()).await?
        }
        #[cfg(feature = "sql")]
        provider if [MYSQL_SOURCE_NAME, SQLITE_SOURCE_NAME, MSSQL_SOURCE_NAME].contains(&provider.as_str()) => {
            SqlMigrationConnector::new(&source.url().value).await?
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    };

    let api = api::MigrationApi::new(connector).await?;

    Ok(Arc::new(api))
}

/// Create the database referenced by the passed in Prisma schema.
pub async fn create_database(schema: &str) -> CoreResult<String> {
    let config = parse_configuration(schema)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::Generic(anyhow!("There is no datasource in the schema.")))?;

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
            Ok(SqlMigrationConnector::create_database(&source.url().value).await?)
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Drop the database referenced by the passed in Prisma schema.
pub async fn drop_database(schema: &str) -> CoreResult<()> {
    let config = parse_configuration(schema)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::Generic(anyhow!("There is no datasource in the schema.")))?;

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
            Ok(SqlMigrationConnector::drop_database(&source.url().value).await?)
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Database setup for connector-test-kit.
pub async fn qe_setup(prisma_schema: &str) -> CoreResult<()> {
    let config = parse_configuration(prisma_schema)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::Generic(anyhow!("There is no datasource in the schema.")))?;

    let connector = match &source.active_provider {
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
            SqlMigrationConnector::new(&source.url().value).await?
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    };
    let engine = MigrationEngine::new(connector).await?;

    // 2. create the database schema for given Prisma schema
    let schema_push_input = SchemaPushInput {
        schema: prisma_schema.to_string(),
        assume_empty: true,
        force: true,
    };
    SchemaPushCommand::execute(&schema_push_input, &engine).await?;

    Ok(())
}

fn parse_configuration(datamodel: &str) -> CoreResult<Configuration> {
    datamodel::parse_configuration(&datamodel)
        .map(|validated_config| validated_config.subject)
        .map_err(|err| CoreError::ReceivedBadDatamodel(err.to_pretty_string("schema.prisma", datamodel)))
}

pub(crate) fn parse_datamodel(datamodel: &str) -> CoreResult<Datamodel> {
    datamodel::parse_datamodel(&datamodel)
        .map(|d| d.subject)
        .map_err(|err| CoreError::ReceivedBadDatamodel(err.to_pretty_string("schema.prisma", datamodel)))
}
