#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms, unsafe_code)]
#![allow(clippy::derive_partial_eq_without_eq)]

pub mod builtin_connectors;
pub mod datamodel_connector;

/// `mcf`: Turns a collection of `configuration::Datasource` and `configuration::Generator` into a
/// JSON representation. This is the `get_config()` representation.
pub mod mcf;

mod common;
mod configuration;
mod reformat;
mod set_config_dir;
mod validate;

use std::sync::Arc;

pub use crate::{
    common::{PreviewFeature, PreviewFeatures, ALL_PREVIEW_FEATURES},
    configuration::{
        Configuration, Datasource, DatasourceConnectorData, Generator, GeneratorConfigValue, StringFromEnvVar,
    },
    reformat::{reformat, reformat_multiple, reformat_validated_schema_into_single},
};
pub use diagnostics;
pub use parser_database::{self, is_reserved_type_name};
pub use schema_ast;
pub use set_config_dir::set_config_dir;

use self::validate::{datasource_loader, generator_loader};
use diagnostics::Diagnostics;
use parser_database::{ast, Files, ParserDatabase, SourceFile};

/// The collection of all available connectors.
pub type ConnectorRegistry<'a> = &'a [&'static dyn datamodel_connector::Connector];

pub struct ValidatedSchema {
    pub configuration: Configuration,
    pub db: parser_database::ParserDatabase,
    pub connector: &'static dyn datamodel_connector::Connector,
    pub diagnostics: Diagnostics,
    relation_mode: datamodel_connector::RelationMode,
}

impl std::fmt::Debug for ValidatedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<Prisma schema>")
    }
}

impl ValidatedSchema {
    pub fn relation_mode(&self) -> datamodel_connector::RelationMode {
        self.relation_mode
    }

    pub fn render_own_diagnostics(&self) -> String {
        self.db.render_diagnostics(&self.diagnostics)
    }
}

/// The most general API for dealing with Prisma schemas. It accumulates what analysis and
/// validation information it can, and returns it along with any error and warning diagnostics.
pub fn validate(file: SourceFile, connectors: ConnectorRegistry<'_>) -> ValidatedSchema {
    let mut diagnostics = Diagnostics::new();
    let db = ParserDatabase::new_single_file(file, &mut diagnostics);
    let configuration = validate_configuration(db.ast_assert_single(), &mut diagnostics, connectors);
    let datasources = &configuration.datasources;
    let out = validate::validate(db, datasources, configuration.preview_features(), diagnostics);

    ValidatedSchema {
        diagnostics: out.diagnostics,
        configuration,
        connector: out.connector,
        db: out.db,
        relation_mode: out.relation_mode,
    }
}

/// The most general API for dealing with Prisma schemas. It accumulates what analysis and
/// validation information it can, and returns it along with any error and warning diagnostics.
pub fn validate_multi_file(files: &[(String, SourceFile)], connectors: ConnectorRegistry<'_>) -> ValidatedSchema {
    assert!(
        !files.is_empty(),
        "psl::validate_multi_file() must be called with at least one file"
    );
    let mut diagnostics = Diagnostics::new();
    let db = ParserDatabase::new(files, &mut diagnostics);

    // TODO: the bulk of configuration block analysis should be part of ParserDatabase::new().
    let mut configuration = Configuration::default();
    for ast in db.iter_asts() {
        let new_config = validate_configuration(ast, &mut diagnostics, connectors);

        configuration.extend(new_config);
    }

    let datasources = &configuration.datasources;
    let out = validate::validate(db, datasources, configuration.preview_features(), diagnostics);

    ValidatedSchema {
        diagnostics: out.diagnostics,
        configuration,
        connector: out.connector,
        db: out.db,
        relation_mode: out.relation_mode,
    }
}

/// Retrieves a Prisma schema without validating it.
/// You should only use this method when actually validating the schema is too expensive
/// computationally or in terms of bundle size (e.g., for `query-engine-wasm`).
pub fn parse_without_validation(file: SourceFile, connectors: ConnectorRegistry<'_>) -> ValidatedSchema {
    let mut diagnostics = Diagnostics::new();
    let db = ParserDatabase::new_single_file(file, &mut diagnostics);
    let configuration = validate_configuration(db.ast_assert_single(), &mut diagnostics, connectors);
    let datasources = &configuration.datasources;
    let out = validate::parse_without_validation(db, datasources);

    ValidatedSchema {
        diagnostics,
        configuration,
        connector: out.connector,
        db: out.db,
        relation_mode: out.relation_mode,
    }
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(
    schema: &str,
    connectors: ConnectorRegistry<'_>,
) -> Result<Configuration, diagnostics::Diagnostics> {
    let source_file = SourceFile::new_allocated(Arc::from(schema.to_owned().into_boxed_str()));
    let (_, out, mut diagnostics) =
        error_tolerant_parse_configuration(&[("schema.prisma".into(), source_file)], connectors);
    diagnostics.to_result().map(|_| out)
}

pub fn parse_configuration_multi_file(
    files: &[(String, SourceFile)],
    connectors: ConnectorRegistry<'_>,
) -> Result<(Files, Configuration), (Files, diagnostics::Diagnostics)> {
    let (files, configuration, mut diagnostics) = error_tolerant_parse_configuration(files, connectors);
    match diagnostics.to_result() {
        Ok(_) => Ok((files, configuration)),
        Err(err) => Err((files, err)),
    }
}

pub fn error_tolerant_parse_configuration(
    files: &[(String, SourceFile)],
    connectors: ConnectorRegistry<'_>,
) -> (Files, Configuration, Diagnostics) {
    let mut diagnostics = Diagnostics::default();
    let mut configuration = Configuration::default();

    let asts = Files::new(files, &mut diagnostics);

    for (_, _, _, ast) in asts.iter() {
        let out = validate_configuration(ast, &mut diagnostics, connectors);
        configuration.extend(out);
    }

    (asts, configuration, diagnostics)
}

fn validate_configuration(
    schema_ast: &ast::SchemaAst,
    diagnostics: &mut Diagnostics,
    connectors: ConnectorRegistry<'_>,
) -> Configuration {
    let generators = generator_loader::load_generators_from_ast(schema_ast, diagnostics);
    let datasources = datasource_loader::load_datasources_from_ast(schema_ast, diagnostics, connectors);

    Configuration::new(generators, datasources, diagnostics.warnings().to_owned())
}
