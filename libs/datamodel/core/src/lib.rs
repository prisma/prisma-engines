#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms, unsafe_code)]

pub mod common;

/// `mcf`: Turns a collection of `configuration::Datasource` and `configuration::Generator` into a
/// JSON representation. This is the `get_config()` representation.
pub mod mcf;

mod configuration;
mod reformat;
mod validate;

pub use crate::{
    configuration::{Configuration, Datasource, Generator, StringFromEnvVar},
    reformat::reformat,
};
pub use datamodel_connector;
pub use diagnostics;
pub use parser_database::{self, is_reserved_type_name};

use self::{
    common::preview_features::PreviewFeature,
    validate::{DatasourceLoader, GeneratorLoader},
};
use diagnostics::Diagnostics;
use parser_database::{ast, ParserDatabase, SourceFile};

pub mod builtin_connectors {
    pub use mongodb_datamodel_connector::*;
    pub use sql_datamodel_connector::*;

    pub(crate) type ConnectorRegistry = &'static [&'static dyn datamodel_connector::Connector];
    pub(crate) const BUILTIN_CONNECTORS: ConnectorRegistry = &[POSTGRES, MYSQL, SQLITE, MSSQL, COCKROACH, MONGODB];
}

pub struct ValidatedSchema {
    pub configuration: Configuration,
    pub db: parser_database::ParserDatabase,
    pub connector: &'static dyn datamodel_connector::Connector,
    pub diagnostics: Diagnostics,
    referential_integrity: datamodel_connector::ReferentialIntegrity,
}

impl ValidatedSchema {
    pub fn referential_integrity(&self) -> datamodel_connector::ReferentialIntegrity {
        self.referential_integrity
    }
}

pub fn parse_schema_parserdb(file: impl Into<SourceFile>) -> Result<ValidatedSchema, String> {
    let mut schema = validate(file.into());
    schema
        .diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", schema.db.source()))?;
    Ok(schema)
}

/// The most general API for dealing with Prisma schemas. It accumulates what analysis and
/// validation information it can, and returns it along with any error and warning diagnostics.
pub fn validate(file: SourceFile) -> ValidatedSchema {
    let mut diagnostics = Diagnostics::new();
    let db = ParserDatabase::new(file, &mut diagnostics);
    let configuration = validate_configuration(db.ast(), &mut diagnostics, builtin_connectors::BUILTIN_CONNECTORS);
    let datasources = &configuration.datasources;
    let out = validate::validate(db, datasources, configuration.preview_features(), diagnostics);

    ValidatedSchema {
        diagnostics: out.diagnostics,
        configuration,
        connector: out.connector,
        db: out.db,
        referential_integrity: out.referential_integrity,
    }
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(schema: &str) -> Result<Configuration, diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let ast = schema_ast::parse_schema(schema, &mut diagnostics);
    let out = validate_configuration(&ast, &mut diagnostics, builtin_connectors::BUILTIN_CONNECTORS);
    diagnostics.to_result().map(|_| out)
}

fn validate_configuration(
    schema_ast: &ast::SchemaAst,
    diagnostics: &mut Diagnostics,
    connectors: builtin_connectors::ConnectorRegistry,
) -> Configuration {
    let generators = GeneratorLoader::load_generators_from_ast(schema_ast, diagnostics);
    let preview_features = generators.iter().filter_map(|gen| gen.preview_features).collect();
    let datasources = DatasourceLoader.load_datasources_from_ast(schema_ast, preview_features, diagnostics, connectors);

    Configuration {
        generators,
        datasources,
        warnings: diagnostics.warnings().to_owned(),
    }
}
