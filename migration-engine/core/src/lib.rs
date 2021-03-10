#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! The top-level library crate for the migration engine.

pub mod api;
pub mod commands;

mod core_error;

pub use api::GenericApi;
pub use commands::SchemaPushInput;
pub use core_error::{CoreError, CoreResult};

use datamodel::{
    common::provider_names::{
        MONGODB_SOURCE_NAME, MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME,
    },
    dml::Datamodel,
    Configuration,
};
use migration_connector::{features, ConnectorError};
use mongodb_migration_connector::MongoDbMigrationConnector;
use sql_migration_connector::SqlMigrationConnector;
use user_facing_errors::{
    common::{InvalidDatabaseString, SchemaParserError},
    migration_engine::DeprecatedProviderArray,
    KnownError,
};

/// Top-level constructor for the migration engine API.
pub async fn migration_api(datamodel: &str) -> CoreResult<Box<dyn api::GenericApi>> {
    let config = parse_configuration(datamodel)?;
    let features = features::from_config(&config);

    let source = config
        .datasources
        .first()
        .map(|source| match source.provider.as_slice() {
            [_] => Ok(source),
            [] => Err(CoreError::from_message(
                "There is no provider in the datasource.".into(),
            )),
            _ => Err(CoreError::user_facing_error(DeprecatedProviderArray)),
        })
        .unwrap_or_else(|| Err(CoreError::from_message("There is no datasource in the schema.".into())))?;

    match &source.active_provider {
        #[cfg(feature = "sql")]
        provider if POSTGRES_SOURCE_NAME == provider => {
            let database_str = &source.url().value;

            let mut u = url::Url::parse(database_str).map_err(|err| {
                let details = user_facing_errors::quaint::invalid_url_description(
                    database_str,
                    &format!("Error parsing connection string: {}", err),
                );

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

            let connector = SqlMigrationConnector::new(
                u.as_str(),
                features,
                source.shadow_database_url.as_ref().map(|url| url.value.clone()),
            )
            .await?;

            Ok(Box::new(connector))
        }
        #[cfg(feature = "sql")]
        provider if [MYSQL_SOURCE_NAME, SQLITE_SOURCE_NAME, MSSQL_SOURCE_NAME].contains(&provider.as_str()) => {
            let connector = SqlMigrationConnector::new(
                &source.url().value,
                features,
                source.shadow_database_url.as_ref().map(|url| url.value.clone()),
            )
            .await?;

            Ok(Box::new(connector))
        }
        #[cfg(feature = "mongodb")]
        provider if provider.as_str() == MONGODB_SOURCE_NAME => {
            let connector = MongoDbMigrationConnector::new(&source.url().value, features).await?;
            Ok(Box::new(connector))
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Create the database referenced by the passed in Prisma schema.
pub async fn create_database(schema: &str) -> CoreResult<String> {
    let config = parse_configuration(schema)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::from_message("There is no datasource in the schema.".into()))?;

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
        .ok_or_else(|| CoreError::from_message("There is no datasource in the schema.".into()))?;

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
    let features = features::from_config(&config);

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::from_message("There is no datasource in the schema.".into()))?;

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
            SqlMigrationConnector::qe_setup(&source.url().value).await?;
            Box::new(SqlMigrationConnector::new(&source.url().value, features, None).await?)
        }
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

fn parse_configuration(datamodel: &str) -> CoreResult<Configuration> {
    datamodel::parse_configuration(&datamodel)
        .map(|validated_config| validated_config.subject)
        .map_err(|err| {
            CoreError::user_facing_error(SchemaParserError {
                full_error: err.to_pretty_string("schema.prisma", datamodel),
            })
        })
}

fn parse_datamodel(datamodel: &str) -> CoreResult<Datamodel> {
    datamodel::parse_datamodel(&datamodel)
        .map(|d| d.subject)
        .map_err(|err| {
            CoreError::user_facing_error(SchemaParserError {
                full_error: err.to_pretty_string("schema.prisma", datamodel),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use user_facing_errors::UserFacingError;

    #[tokio::test]
    async fn migration_api_with_a_provider_array_returns_a_user_facing_error() {
        let datamodel = r#"
            datasource dbs {
                provider = ["sqlite", "mysql"]
                url = "file:dev.db"
            }
        "#;

        let err = migration_api(datamodel)
            .await
            .map(drop)
            .unwrap_err()
            .to_user_facing()
            .unwrap_known();

        assert_eq!(err.error_code, DeprecatedProviderArray::ERROR_CODE);
    }
}
