#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects
#![allow(clippy::derive_partial_eq_without_eq)]

//! The top-level library crate for the schema engine.

use enumflags2::BitFlags;
pub use json_rpc;
use sql_schema_connector::SqlSchemaDialect;

// exposed for tests
#[doc(hidden)]
pub mod commands;

#[doc(hidden)]
pub mod core_error;

mod api;

pub use self::{api::GenericApi, core_error::*};
pub use commands::*;
use json_rpc::types::{SchemaContainer, SchemasContainer, SchemasWithConfigDir};
pub use schema_connector;

use psl::{
    ValidatedSchema, builtin_connectors::BUILTIN_CONNECTORS, datamodel_connector::Flavour, parser_database::SourceFile,
};

/// Creates the [`SqlSchemaDialect`](SqlSchemaDialect) matching the given provider.
pub fn dialect_for_provider(provider: &str) -> CoreResult<Box<dyn schema_connector::SchemaDialect>> {
    let error = Err(CoreError::from_msg(format!(
        "`{provider}` is not a supported connector."
    )));

    if let Some(connector) = BUILTIN_CONNECTORS.iter().find(|c| c.is_provider(provider)) {
        match connector.flavour() {
            Flavour::Cockroach => Ok(Box::new(SqlSchemaDialect::cockroach())),
            Flavour::Postgres => Ok(Box::new(SqlSchemaDialect::postgres())),
            Flavour::Sqlite => Ok(Box::new(SqlSchemaDialect::sqlite())),

            // TODO: enable these in Prisma 6.7.0
            Flavour::Mongo => error,
            Flavour::Sqlserver => error,
            Flavour::Mysql => error,
        }
    } else {
        error
    }
}

/// Extracts the database namespaces from the given schema files.
pub fn extract_namespaces(
    files: &[(String, psl::SourceFile)],
    namespaces: &mut Vec<String>,
    preview_features: &mut BitFlags<psl::PreviewFeature>,
) {
    let validated_schema = psl::validate_multi_file(files);

    for (namespace, _span) in validated_schema
        .configuration
        .datasources
        .iter()
        .flat_map(|ds| ds.namespaces.iter())
    {
        namespaces.push(namespace.clone());
    }

    for generator in &validated_schema.configuration.generators {
        *preview_features |= generator.preview_features.unwrap_or_default();
    }
}

fn parse_schema_multi(files: &[(String, SourceFile)]) -> CoreResult<ValidatedSchema> {
    psl::parse_schema_multi(files).map_err(CoreError::new_schema_parser_error)
}

/// Wrapper trait for `SchemaContainer` and related types.
pub trait SchemaContainerExt {
    /// Converts self into a suitable input for the PSL parser.
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
