#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects
#![allow(clippy::derive_partial_eq_without_eq)]

//! The top-level library crate for the schema engine.

pub use json_rpc;

// exposed for tests
#[doc(hidden)]
pub mod commands;

mod api;
mod core_error;
mod rpc;
mod state;
mod timings;

pub use self::{api::GenericApi, core_error::*, rpc::rpc_api, timings::TimingsLayer};
use json_rpc::types::{SchemaContainer, SchemasContainer, SchemasWithConfigDir};
pub use schema_connector;

use enumflags2::BitFlags;
use mongodb_schema_connector::{MongoDbSchemaConnector, MongoDbSchemaFlavour};
use psl::{
    builtin_connectors::*, datamodel_connector::Flavour, parser_database::SourceFile, Datasource, PreviewFeature,
    ValidatedSchema,
};
use schema_connector::ConnectorParams;
use sql_schema_connector::{SqlSchemaConnector, SqlSchemaDialect};
use std::{env, path::Path};
use user_facing_errors::common::InvalidConnectionString;

fn parse_schema_multi(files: &[(String, SourceFile)]) -> CoreResult<ValidatedSchema> {
    psl::parse_schema_multi(files).map_err(CoreError::new_schema_parser_error)
}

fn connector_for_connection_string(
    connection_string: String,
    shadow_database_connection_string: Option<String>,
    preview_features: BitFlags<PreviewFeature>,
) -> CoreResult<Box<dyn schema_connector::SchemaConnector>> {
    match connection_string.split(':').next() {
        Some("postgres") | Some("postgresql") | Some("prisma+postgres") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            Ok(Box::new(SqlSchemaConnector::new_postgres_like(params)?))
        }
        Some("file") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            Ok(Box::new(SqlSchemaConnector::new_sqlite(params)?))
        }
        Some("mysql") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            Ok(Box::new(SqlSchemaConnector::new_mysql(params)?))
        }
        Some("sqlserver") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            Ok(Box::new(SqlSchemaConnector::new_mssql(params)?))
        }
        Some("mongodb+srv") | Some("mongodb") => {
            let params = ConnectorParams {
                connection_string,
                preview_features,
                shadow_database_connection_string,
            };
            let connector = MongoDbSchemaConnector::new(params);
            Ok(Box::new(connector))
        }
        Some(_other) => Err(CoreError::url_parse_error("The scheme is not recognized")),
        None => Err(CoreError::user_facing(InvalidConnectionString {
            details: String::new(),
        })),
    }
}

/// Same as schema_to_connector, but it will only read the provider, not the connector params.
fn schema_to_dialect(files: &[(String, SourceFile)]) -> CoreResult<Box<dyn schema_connector::SchemaDialect>> {
    let (_, config) = psl::parse_configuration_multi_file(files)
        .map_err(|(files, err)| CoreError::new_schema_parser_error(files.render_diagnostics(&err)))?;

    let preview_features = config.preview_features();
    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    if let Ok(connection_string) = source.load_direct_url(|key| env::var(key).ok()) {
        let connector_params = ConnectorParams {
            connection_string,
            preview_features,
            shadow_database_connection_string: source.load_shadow_database_url().ok().flatten(),
        };
        let conn = connector_for_provider(source.active_provider, connector_params)?;
        Ok(conn.schema_dialect())
    } else {
        dialect_for_provider(source.active_provider)
    }
}

/// Go from a schema to a connector
fn schema_to_connector(
    files: &[(String, SourceFile)],
    config_dir: Option<&Path>,
) -> CoreResult<Box<dyn schema_connector::SchemaConnector>> {
    let (source, url, preview_features, shadow_database_url) = parse_configuration_multi(files)?;

    let url = config_dir
        .map(|config_dir| psl::set_config_dir(source.active_connector.flavour(), config_dir, &url).into_owned())
        .unwrap_or(url);

    let params = ConnectorParams {
        connection_string: url,
        preview_features,
        shadow_database_connection_string: shadow_database_url,
    };

    connector_for_provider(source.active_provider, params)
}

fn connector_for_provider(
    provider: &str,
    params: ConnectorParams,
) -> CoreResult<Box<dyn schema_connector::SchemaConnector>> {
    if let Some(connector) = BUILTIN_CONNECTORS.iter().find(|c| c.is_provider(provider)) {
        match connector.flavour() {
            Flavour::Cockroach => Ok(Box::new(SqlSchemaConnector::new_cockroach(params)?)),
            Flavour::Mongo => Ok(Box::new(MongoDbSchemaConnector::new(params))),
            Flavour::Sqlserver => Ok(Box::new(SqlSchemaConnector::new_mssql(params)?)),
            Flavour::Mysql => Ok(Box::new(SqlSchemaConnector::new_mysql(params)?)),
            Flavour::Postgres => Ok(Box::new(SqlSchemaConnector::new_postgres(params)?)),
            Flavour::Sqlite => Ok(Box::new(SqlSchemaConnector::new_sqlite(params)?)),
        }
    } else {
        Err(CoreError::from_msg(format!(
            "`{provider}` is not a supported connector."
        )))
    }
}

fn dialect_for_provider(provider: &str) -> CoreResult<Box<dyn schema_connector::SchemaDialect>> {
    if let Some(connector) = BUILTIN_CONNECTORS.iter().find(|c| c.is_provider(provider)) {
        match connector.flavour() {
            Flavour::Cockroach => Ok(Box::new(SqlSchemaDialect::cockroach())),
            Flavour::Mongo => Ok(Box::new(MongoDbSchemaFlavour)),
            Flavour::Sqlserver => Ok(Box::new(SqlSchemaDialect::mssql())),
            Flavour::Mysql => Ok(Box::new(SqlSchemaDialect::mysql())),
            Flavour::Postgres => Ok(Box::new(SqlSchemaDialect::postgres())),
            Flavour::Sqlite => Ok(Box::new(SqlSchemaDialect::sqlite())),
        }
    } else {
        Err(CoreError::from_msg(format!(
            "`{provider}` is not a supported connector."
        )))
    }
}

/// Top-level constructor for the schema engine API.
pub fn schema_api(
    datamodel: Option<String>,
    host: Option<std::sync::Arc<dyn schema_connector::ConnectorHost>>,
) -> CoreResult<Box<dyn api::GenericApi>> {
    // Eagerly load the default schema, for validation errors.
    if let Some(datamodel) = &datamodel {
        parse_configuration(datamodel)?;
    }

    let datamodel = datamodel.map(|datamodel| vec![("schema.prisma".to_owned(), SourceFile::from(datamodel))]);
    let state = state::EngineState::new(datamodel, host);
    Ok(Box::new(state))
}

fn parse_configuration(datamodel: &str) -> CoreResult<(Datasource, String, BitFlags<PreviewFeature>, Option<String>)> {
    let config = psl::parse_configuration(datamodel)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    extract_configuration(config, |err| {
        CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel))
    })
}

fn parse_configuration_multi(
    files: &[(String, SourceFile)],
) -> CoreResult<(Datasource, String, BitFlags<PreviewFeature>, Option<String>)> {
    let (files, config) = psl::parse_configuration_multi_file(files)
        .map_err(|(files, err)| CoreError::new_schema_parser_error(files.render_diagnostics(&err)))?;

    extract_configuration(config, |err| {
        CoreError::new_schema_parser_error(files.render_diagnostics(&err))
    })
}

fn extract_configuration(
    config: psl::Configuration,
    mut err_handler: impl FnMut(psl::Diagnostics) -> CoreError,
) -> CoreResult<(Datasource, String, BitFlags<PreviewFeature>, Option<String>)> {
    let preview_features = config.preview_features();

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    let url = source
        .load_direct_url(|key| env::var(key).ok())
        .map_err(&mut err_handler)?;

    let shadow_database_url = source.load_shadow_database_url().map_err(err_handler)?;

    Ok((source, url, preview_features, shadow_database_url))
}

trait SchemaContainerExt {
    fn to_psl_input(self) -> Vec<(String, SourceFile)>;
}

impl SchemaContainerExt for SchemasContainer {
    fn to_psl_input(self) -> Vec<(String, SourceFile)> {
        self.files.to_psl_input()
    }
}

impl SchemaContainerExt for &SchemasContainer {
    fn to_psl_input(self) -> Vec<(String, SourceFile)> {
        (&self.files).to_psl_input()
    }
}

impl SchemaContainerExt for Vec<SchemaContainer> {
    fn to_psl_input(self) -> Vec<(String, SourceFile)> {
        self.into_iter()
            .map(|container| (container.path, SourceFile::from(container.content)))
            .collect()
    }
}

impl SchemaContainerExt for Vec<&SchemaContainer> {
    fn to_psl_input(self) -> Vec<(String, SourceFile)> {
        self.into_iter()
            .map(|container| (container.path.clone(), SourceFile::from(&container.content)))
            .collect()
    }
}

impl SchemaContainerExt for &Vec<SchemaContainer> {
    fn to_psl_input(self) -> Vec<(String, SourceFile)> {
        self.iter()
            .map(|container| (container.path.clone(), SourceFile::from(&container.content)))
            .collect()
    }
}

impl SchemaContainerExt for &SchemasWithConfigDir {
    fn to_psl_input(self) -> Vec<(String, SourceFile)> {
        (&self.files).to_psl_input()
    }
}
