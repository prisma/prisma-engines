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
mod migration_schema_cache;

pub use self::{api::GenericApi, core_error::*};
pub use commands::*;
use json_rpc::types::{SchemaContainer, SchemasContainer, SchemasWithConfigDir};
pub use migration_schema_cache::MigrationSchemaCache;
pub use schema_connector;

use psl::{
    ValidatedSchema,
    builtin_connectors::BUILTIN_CONNECTORS,
    datamodel_connector::Flavour,
    parser_database::{ExtensionTypes, SourceFile},
};

/// Creates the [`SqlSchemaDialect`](SqlSchemaDialect) matching the given provider.
pub fn dialect_for_provider(provider: &str) -> CoreResult<Box<dyn schema_connector::SchemaDialect>> {
    let error = || {
        Err(CoreError::from_msg(format!(
            "`{provider}` is not a supported connector."
        )))
    };

    if let Some(connector) = BUILTIN_CONNECTORS.iter().find(|c| c.is_provider(provider)) {
        match connector.flavour() {
            #[cfg(feature = "sqlite")]
            Flavour::Sqlite => Ok(Box::new(SqlSchemaDialect::sqlite())),

            #[cfg(feature = "postgresql")]
            Flavour::Postgres => Ok(Box::new(SqlSchemaDialect::postgres())),

            #[cfg(feature = "postgresql")]
            Flavour::Cockroach => Ok(Box::new(SqlSchemaDialect::cockroach())),

            #[cfg(feature = "mongodb")]
            Flavour::Mongo => Ok(Box::new(mongodb_schema_connector::MongoDbSchemaDialect)),

            #[cfg(feature = "mssql")]
            Flavour::Sqlserver => Ok(Box::new(SqlSchemaDialect::mssql())),

            #[cfg(feature = "mysql")]
            Flavour::Mysql => Ok(Box::new(SqlSchemaDialect::mysql())),

            #[allow(unreachable_patterns)]
            _ => error(),
        }
    } else {
        error()
    }
}

/// Extracts the database namespaces from the given schema files.
pub fn extract_namespaces(
    files: &[(String, psl::SourceFile)],
    namespaces: &mut Vec<String>,
    preview_features: &mut BitFlags<psl::PreviewFeature>,
) {
    let validated_schema = psl::validate_multi_file_without_extensions(files);

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

fn parse_schema_multi(
    files: &[(String, SourceFile)],
    extension_types: &dyn ExtensionTypes,
) -> CoreResult<ValidatedSchema> {
    psl::parse_schema_multi(files, extension_types).map_err(CoreError::new_schema_parser_error)
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
