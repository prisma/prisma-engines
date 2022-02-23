#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects

//! The top-level library crate for the migration engine.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

// exposed for tests
#[doc(hidden)]
pub mod commands;

mod api;
mod core_error;
mod rpc;
mod state;

pub use self::{api::GenericApi, core_error::*, rpc::rpc_api};
pub use migration_connector;

use datamodel::ValidatedSchema;
use datamodel::{
    common::{
        preview_features::PreviewFeature,
        provider_names::{
            MONGODB_SOURCE_NAME, MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME,
        },
    },
    Datasource,
};
use enumflags2::BitFlags;
use migration_connector::ConnectorParams;
use mongodb_migration_connector::MongoDbMigrationConnector;
use sql_migration_connector::SqlMigrationConnector;
use std::env;
use user_facing_errors::common::InvalidConnectionString;

fn parse_schema(schema: &str) -> CoreResult<ValidatedSchema> {
    datamodel::parse_schema_parserdb(schema).map_err(CoreError::new_schema_parser_error)
}

fn connector_for_connection_string(
    connection_string: String,
    shadow_database_connection_string: Option<String>,
    preview_features: BitFlags<PreviewFeature>,
) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    match connection_string.split(':').next() {
        Some("postgres") | Some("postgresql") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            let mut connector = SqlMigrationConnector::new_postgres();
            connector.set_params(params)?;
            Ok(Box::new(connector))
        }
        // TODO: `sqlite:` connection strings may not work if we try to connect to them, but they
        // seem to be used by some tests in prisma/prisma. They are not tested at all engine-side.
        //
        // Tracking issue: https://github.com/prisma/prisma/issues/11468
        Some("file") | Some("sqlite") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            let mut connector = SqlMigrationConnector::new_sqlite();
            connector.set_params(params)?;
            Ok(Box::new(connector))
        }
        Some("mysql") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            let mut connector = SqlMigrationConnector::new_mysql();
            connector.set_params(params)?;
            Ok(Box::new(connector))
        }
        Some("sqlserver") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            let mut connector = SqlMigrationConnector::new_mssql();
            connector.set_params(params)?;
            Ok(Box::new(connector))
        }
        Some("mongodb+srv") | Some("mongodb") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            let connector = MongoDbMigrationConnector::new(params);
            Ok(Box::new(connector))
        }
        Some(other) => Err(CoreError::url_parse_error(format!(
            "`{other}` is not a known connection URL scheme. Prisma cannot determine the connector."
        ))),
        None => Err(CoreError::user_facing(InvalidConnectionString {
            details: String::new(),
        })),
    }
}

/// Same as schema_to_connector, but it will only read the provider, not the connector params.
fn schema_to_connector_unchecked(schema: &str) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    let config = datamodel::parse_configuration(schema)
        .map(|validated_config| validated_config.subject)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", schema)))?;

    let preview_features = config.preview_features();
    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    let mut connector = connector_for_provider(source.active_provider.as_str())?;

    if let Ok(connection_string) = source.load_url(|key| env::var(key).ok()) {
        connector.set_params(ConnectorParams {
            connection_string,
            preview_features,
            shadow_database_connection_string: source.load_shadow_database_url().ok().flatten(),
        })?;
    }

    Ok(connector)
}

/// Go from a schema to a connector
fn schema_to_connector(schema: &str) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    let (source, url, preview_features, shadow_database_url) = parse_configuration(schema)?;
    let params = ConnectorParams {
        connection_string: url,
        preview_features,
        shadow_database_connection_string: shadow_database_url,
    };

    let mut connector = connector_for_provider(source.active_provider.as_str())?;
    connector.set_params(params)?;
    Ok(connector)
}

fn connector_for_provider(provider: &str) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    match provider {
        POSTGRES_SOURCE_NAME => Ok(Box::new(SqlMigrationConnector::new_postgres())),
        MYSQL_SOURCE_NAME => Ok(Box::new(SqlMigrationConnector::new_mysql())),
        SQLITE_SOURCE_NAME => Ok(Box::new(SqlMigrationConnector::new_sqlite())),
        MSSQL_SOURCE_NAME => Ok(Box::new(SqlMigrationConnector::new_mssql())),
        // TODO: adopt a state machine pattern in the mongo connector too
        MONGODB_SOURCE_NAME => Ok(Box::new(MongoDbMigrationConnector::new(ConnectorParams {
            connection_string: String::new(),
            preview_features: Default::default(),
            shadow_database_connection_string: None,
        }))),
        provider => Err(CoreError::from_msg(format!(
            "`{}` is not a supported connector.",
            provider
        ))),
    }
}

/// Top-level constructor for the migration engine API.
pub fn migration_api(
    datamodel: Option<String>,
    host: Option<std::sync::Arc<dyn migration_connector::ConnectorHost>>,
) -> CoreResult<Box<dyn api::GenericApi>> {
    // Eagerly load the default schema, for validation errors.
    if let Some(datamodel) = &datamodel {
        parse_configuration(datamodel)?;
    }

    let state = state::EngineState::new(datamodel, host);
    Ok(Box::new(state))
}

fn parse_configuration(datamodel: &str) -> CoreResult<(Datasource, String, BitFlags<PreviewFeature>, Option<String>)> {
    let config = datamodel::parse_configuration(datamodel)
        .map(|validated_config| validated_config.subject)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let preview_features = config.preview_features();

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    let url = source
        .load_url(|key| env::var(key).ok())
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let shadow_database_url = source
        .load_shadow_database_url()
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    Ok((source, url, preview_features, shadow_database_url))
}
