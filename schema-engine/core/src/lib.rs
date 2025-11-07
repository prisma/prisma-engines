#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects
#![allow(clippy::derive_partial_eq_without_eq)]

//! The top-level library crate for the schema engine.

// exposed for tests
#[doc(hidden)]
pub mod commands;

pub use ::commands::{CoreError, CoreResult, GenericApi};
pub use json_rpc;

mod core_error;
mod extensions;
mod rpc;
mod state;
mod timings;
mod url;

use crate::url::ValidatedDatasourceUrls;

pub use self::{rpc::RpcApi, timings::TimingsLayer, url::DatasourceUrls};
pub use extensions::{ExtensionType, ExtensionTypeConfig};
use json_rpc::types::{SchemaContainer, SchemasContainer, SchemasWithConfigDir};
pub use schema_connector;

use enumflags2::BitFlags;
use mongodb_schema_connector::MongoDbSchemaConnector;
use psl::{
    Datasource, PreviewFeature, builtin_connectors::*, datamodel_connector::Flavour, parser_database::SourceFile,
};
use schema_connector::ConnectorParams;
use sql_schema_connector::SqlSchemaConnector;
use std::{path::Path, sync::Arc};

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
        Some(_) => Err(CoreError::url_parse_error("The scheme is not recognized")),
        None => Err(CoreError::url_parse_error("Missing URL scheme")),
    }
}

/// Same as schema_to_connector, but it will only read the provider, not the connector params.
/// This uses `schema_files` to read `preview_features` and the `datasource` block.
fn schema_to_dialect(
    schema_files: &[(String, SourceFile)],
    datasource_urls: &ValidatedDatasourceUrls,
) -> CoreResult<Box<dyn schema_connector::SchemaDialect>> {
    let (_, config) = psl::parse_configuration_multi_file(schema_files)
        .map_err(|(files, err)| CoreError::new_schema_parser_error(files.render_diagnostics(&err)))?;

    let preview_features = config.preview_features();
    let datasource = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    let connector_params = ConnectorParams {
        connection_string: datasource_urls.url().to_owned(),
        preview_features,
        shadow_database_connection_string: datasource_urls.shadow_database_url().map(<_>::to_owned),
    };

    let conn = connector_for_provider(datasource.active_provider, connector_params)?;

    Ok(conn.schema_dialect())
}

/// Go from a schema to a connector.
fn schema_to_connector(
    files: &[(String, SourceFile)],
    datasource_urls: &ValidatedDatasourceUrls,
    config_dir: Option<&Path>,
) -> CoreResult<Box<dyn schema_connector::SchemaConnector>> {
    let (datasource, preview_features) = parse_configuration_multi(files)?;

    let (connection_string, shadow_database_connection_string) = if let Some(config_dir) = config_dir {
        let urls = datasource_urls.with_config_dir(datasource.active_connector.flavour(), config_dir);
        (urls.url().to_owned(), urls.shadow_database_url().map(<_>::to_owned))
    } else {
        (
            datasource_urls.url().to_owned(),
            datasource_urls.shadow_database_url().map(<_>::to_owned),
        )
    };

    let params = ConnectorParams {
        connection_string,
        preview_features,
        shadow_database_connection_string,
    };

    connector_for_provider(datasource.active_provider, params)
}

fn initial_datamodel_to_connector(
    initial_datamodel: &psl::ValidatedSchema,
    datasource_urls: &ValidatedDatasourceUrls,
) -> CoreResult<Box<dyn schema_connector::SchemaConnector>> {
    let configuration = &initial_datamodel.configuration;
    let (datasource, preview_features) = extract_configuration_ref(configuration)?;

    let params = ConnectorParams {
        connection_string: datasource_urls.url().to_owned(),
        preview_features,
        shadow_database_connection_string: datasource_urls.shadow_database_url().map(<_>::to_owned),
    };

    connector_for_provider(datasource.active_provider, params)
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

/// Top-level constructor for the schema engine API.
/// This variant does not support extensions.
pub fn schema_api_without_extensions(
    datamodel: Option<String>,
    datasource_urls: DatasourceUrls,
    host: Option<std::sync::Arc<dyn schema_connector::ConnectorHost>>,
) -> CoreResult<Box<dyn GenericApi>> {
    schema_api(
        datamodel,
        datasource_urls,
        host,
        Arc::new(ExtensionTypeConfig::default()),
    )
}

/// Top-level constructor for the schema engine API.
pub fn schema_api(
    datamodel: Option<String>,
    datasource_urls: DatasourceUrls,
    host: Option<std::sync::Arc<dyn schema_connector::ConnectorHost>>,
    extension_config: Arc<ExtensionTypeConfig>,
) -> CoreResult<Box<dyn GenericApi>> {
    // Eagerly load the default schema, for validation errors.
    if let Some(datamodel) = &datamodel {
        parse_configuration(datamodel)?;
    }

    let datamodel = datamodel.map(|datamodel| vec![("schema.prisma".to_owned(), SourceFile::from(datamodel))]);

    let state = state::EngineState::new(datamodel, datasource_urls, host, extension_config);
    Ok(Box::new(state))
}

fn parse_configuration(datamodel: &str) -> CoreResult<(Datasource, BitFlags<PreviewFeature>)> {
    let config = psl::parse_configuration(datamodel)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    extract_configuration(config)
}

fn parse_configuration_multi(files: &[(String, SourceFile)]) -> CoreResult<(Datasource, BitFlags<PreviewFeature>)> {
    let (_, config) = psl::parse_configuration_multi_file(files)
        .map_err(|(files, err)| CoreError::new_schema_parser_error(files.render_diagnostics(&err)))?;

    extract_configuration(config)
}

fn extract_configuration(config: psl::Configuration) -> CoreResult<(Datasource, BitFlags<PreviewFeature>)> {
    let preview_features = config.preview_features();

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    Ok((source, preview_features))
}

fn extract_configuration_ref(config: &psl::Configuration) -> CoreResult<(&Datasource, BitFlags<PreviewFeature>)> {
    let preview_features = config.preview_features();

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    Ok((source, preview_features))
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
