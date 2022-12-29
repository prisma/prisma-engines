#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects
#![allow(clippy::derive_partial_eq_without_eq)]

//! The top-level library crate for the migration engine.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

// exposed for tests
#[doc(hidden)]
pub mod commands;

mod api;
mod core_error;
mod rpc;
mod state;
mod timings;

pub use self::{api::GenericApi, core_error::*, rpc::rpc_api, timings::TimingsLayer};
pub use migration_connector;

use enumflags2::BitFlags;
use migration_connector::ConnectorParams;
use mongodb_migration_connector::MongoDbMigrationConnector;
use psl::{builtin_connectors::*, parser_database::SourceFile, Datasource, PreviewFeature, ValidatedSchema};
use sql_migration_connector::SqlMigrationConnector;
use std::{env, path::Path};
use user_facing_errors::common::InvalidConnectionString;

fn parse_schema(schema: SourceFile) -> CoreResult<ValidatedSchema> {
    psl::parse_schema(schema).map_err(CoreError::new_schema_parser_error)
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
        Some("file") => {
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
    let config = psl::parse_configuration(schema)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", schema)))?;

    let preview_features = config.preview_features();
    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    let mut connector = connector_for_provider(source.active_provider)?;

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
fn schema_to_connector(
    schema: &str,
    config_dir: Option<&Path>,
) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    let (source, url, preview_features, shadow_database_url) = parse_configuration(schema)?;

    let url = config_dir
        .map(|config_dir| source.active_connector.set_config_dir(config_dir, &url).into_owned())
        .unwrap_or(url);

    let params = ConnectorParams {
        connection_string: url,
        preview_features,
        shadow_database_connection_string: shadow_database_url,
    };

    let mut connector = connector_for_provider(source.active_provider)?;
    connector.set_params(params)?;
    Ok(connector)
}

fn connector_for_provider(provider: &str) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    match provider {
        p if POSTGRES.is_provider(p) => Ok(Box::new(SqlMigrationConnector::new_postgres())),
        p if COCKROACH.is_provider(p) => Ok(Box::new(SqlMigrationConnector::new_cockroach())),
        p if MYSQL.is_provider(p) => Ok(Box::new(SqlMigrationConnector::new_mysql())),
        p if SQLITE.is_provider(p) => Ok(Box::new(SqlMigrationConnector::new_sqlite())),
        p if MSSQL.is_provider(p) => Ok(Box::new(SqlMigrationConnector::new_mssql())),
        // TODO: adopt a state machine pattern in the mongo connector too
        p if MONGODB.is_provider(p) => Ok(Box::new(MongoDbMigrationConnector::new(ConnectorParams {
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
    namespaces: Vec<String>,
    host: Option<std::sync::Arc<dyn migration_connector::ConnectorHost>>,
) -> CoreResult<Box<dyn api::GenericApi>> {
    // Eagerly load the default schema, for validation errors.
    if let Some(datamodel) = &datamodel {
        parse_configuration(datamodel)?;
    }

    let state = state::EngineState::new(datamodel, namespaces, host);
    Ok(Box::new(state))
}

fn parse_configuration(datamodel: &str) -> CoreResult<(Datasource, String, BitFlags<PreviewFeature>, Option<String>)> {
    let config = psl::parse_configuration(datamodel)
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
