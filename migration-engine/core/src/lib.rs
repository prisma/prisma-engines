#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects

//! The top-level library crate for the migration engine.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

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
use user_facing_errors::{common::InvalidConnectionString, KnownError};

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
            let mut params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            disable_postgres_statement_cache(&mut params)?;
            let connector = SqlMigrationConnector::new(params)?;
            Ok(Box::new(connector))
        }
        // TODO: `sqlite:` connection strings may not work if we try to connect to them, but they
        // seem to be used by some tests in prisma/prisma. They are not tested at all engine-side.
        //
        // Tracking issue: https://github.com/prisma/prisma/issues/11468
        Some("file") | Some("mysql") | Some("sqlserver") | Some("sqlite") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            let connector = SqlMigrationConnector::new(params)?;
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

/// Same as schema_to_connector, but it will not resolve env vars in the datasource.
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

    let params = ConnectorParams {
        // TODO: make ConnectorParams optional in connector construction so we don't need to do
        // this
        connection_string: source.active_provider.to_owned() + ":",
        preview_features,
        shadow_database_connection_string: None,
    };

    connector_for_provider(source.active_provider.as_str(), params)
}

/// Go from a schema to a connector
fn schema_to_connector(schema: &str) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    let (source, url, preview_features, shadow_database_url) = parse_configuration(schema)?;
    let params = ConnectorParams {
        connection_string: url,
        preview_features,
        shadow_database_connection_string: shadow_database_url,
    };

    connector_for_provider(source.active_provider.as_str(), params)
}

fn connector_for_provider(
    provider: &str,
    mut params: ConnectorParams,
) -> CoreResult<Box<dyn migration_connector::MigrationConnector>> {
    match provider {
        POSTGRES_SOURCE_NAME => {
            disable_postgres_statement_cache(&mut params)?;
            let connector = SqlMigrationConnector::new(params)?;
            Ok(Box::new(connector))
        }
        MYSQL_SOURCE_NAME | SQLITE_SOURCE_NAME | MSSQL_SOURCE_NAME => {
            let connector = SqlMigrationConnector::new(params)?;
            Ok(Box::new(connector))
        }
        MONGODB_SOURCE_NAME => Ok(Box::new(MongoDbMigrationConnector::new(params))),
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

fn disable_postgres_statement_cache(connector_params: &mut ConnectorParams) -> CoreResult<()> {
    let mut u = url::Url::parse(&connector_params.connection_string).map_err(|err| {
        let details = user_facing_errors::quaint::invalid_connection_string_description(&format!(
            "Error parsing connection string: {}",
            err
        ));

        CoreError::from(KnownError::new(InvalidConnectionString { details }))
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

    connector_params.connection_string = u.to_string();
    Ok(())
}
