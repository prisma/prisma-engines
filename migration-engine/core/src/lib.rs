#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! The top-level library crate for the migration engine.

pub mod api;
pub mod commands;
pub mod qe_setup;

mod core_error;

pub use api::GenericApi;
pub use commands::SchemaPushInput;
pub use core_error::{CoreError, CoreResult};

use datamodel::{
    common::provider_names::{MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    dml::Datamodel,
    Configuration, Datasource,
};
use migration_connector::ConnectorError;
use sql_migration_connector::SqlMigrationConnector;
use user_facing_errors::{common::InvalidDatabaseString, KnownError};

#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
#[cfg(feature = "mongodb")]
use mongodb_migration_connector::MongoDbMigrationConnector;

/// Top-level constructor for the migration engine API.
pub async fn migration_api(datamodel: &str) -> CoreResult<Box<dyn api::GenericApi>> {
    let (source, url, shadow_database_url) = parse_configuration(datamodel)?;

    match source.active_provider.as_str() {
        #[cfg(feature = "sql")]
        POSTGRES_SOURCE_NAME => {
            let mut u = url::Url::parse(&url).map_err(|err| {
                let details = user_facing_errors::quaint::invalid_url_description(&format!(
                    "Error parsing connection string: {}",
                    err
                ));

                ConnectorError::from(KnownError::new(InvalidDatabaseString { details }))
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

            let connector = SqlMigrationConnector::new(u.as_str(), shadow_database_url).await?;

            Ok(Box::new(connector))
        }
        #[cfg(feature = "sql")]
        MYSQL_SOURCE_NAME | SQLITE_SOURCE_NAME | MSSQL_SOURCE_NAME => {
            let connector = SqlMigrationConnector::new(&url, shadow_database_url).await?;

            Ok(Box::new(connector))
        }
        #[cfg(feature = "mongodb")]
        MONGODB_SOURCE_NAME => Ok(Box::new(MongoDbMigrationConnector::new(&url).await?)),
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Create the database referenced by the passed in Prisma schema.
pub async fn create_database(schema: &str) -> CoreResult<String> {
    let (source, url, _shadow_database_url) = parse_configuration(schema)?;

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
            Ok(SqlMigrationConnector::create_database(&url).await?)
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Drop the database referenced by the passed in Prisma schema.
pub async fn drop_database(schema: &str) -> CoreResult<()> {
    let (source, url, _shadow_database_url) = parse_configuration(schema)?;

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
            Ok(SqlMigrationConnector::drop_database(&url).await?)
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

fn parse_configuration(datamodel: &str) -> CoreResult<(Datasource, String, Option<String>)> {
    let config = datamodel::parse_configuration(&datamodel)
        .map(|validated_config| validated_config.subject)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let url = config.datasources[0]
        .load_url()
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let shadow_database_url = config.datasources[0]
        .load_shadow_database_url()
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    Ok((source, url, shadow_database_url))
}

fn parse_schema(schema: &str) -> CoreResult<(Configuration, Datamodel)> {
    datamodel::parse_schema(&schema)
        .map(|d| d.subject)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", schema)))
}
