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

pub use crate::{
    common::{PreviewFeature, PreviewFeatures, ALL_PREVIEW_FEATURES},
    configuration::{
        Configuration, Datasource, DatasourceConnectorData, Generator, GeneratorConfigValue, StringFromEnvVar,
    },
    reformat::reformat,
};
pub use diagnostics;
pub use parser_database::{self, is_reserved_type_name};
pub use schema_ast;
pub use set_config_dir::set_config_dir;

use self::validate::{datasource_loader, generator_loader};
use diagnostics::{DatamodelWarning, Diagnostics};
use parser_database::{ast, ParserDatabase, SourceFile};

/// The collection of all available connectors.
pub type ConnectorRegistry<'a> = &'a [&'static dyn datamodel_connector::Connector];

/// The collection of all available validated connectors.
pub type ValidatedConnectorRegistry<'a> = &'a [&'static dyn datamodel_connector::ValidatedConnector];

// `libquery` needs to:
// - read the `preview_features` bitflags
// - read the `relation_mode` enum
// - read the active `connector`
// - read the `db` parser database (which is the most problematic so far)
pub struct ValidatedSchema {
    // `libquery` uses `configuration` to:
    // - read the `preview_features` bitflags
    pub configuration: Configuration,

    // `libquery` uses `db` to:
    // - build the query schema lazily (by `walk`-ing the `db`)
    // - read models, finding them by name
    // - read model counts
    pub db: parser_database::ParserDatabase,

    // `libquery` uses `connector` to:
    // - read the `capabilities` bitflags
    // - parse native types
    // - check support for referential actions
    // - read the `provider` string
    pub connector: &'static dyn datamodel_connector::Connector,

    // `libquery` uses `connector` to:
    // - customize the behavior of referential actions
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
}

/// The most general API for dealing with Prisma schemas. It accumulates what analysis and
/// validation information it can, and returns it along with any error and warning diagnostics.
pub fn validate(file: SourceFile, connectors: ConnectorRegistry<'_>) -> (ValidatedSchema, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let db = ParserDatabase::new(file, &mut diagnostics);
    let (configuration, _) = validate_configuration(db.ast(), &mut diagnostics, connectors);
    let datasources = &configuration.datasources;
    let out = validate::validate(db, datasources, configuration.preview_features(), diagnostics);

    (
        ValidatedSchema {
            configuration,
            connector: out.connector,
            db: out.db,
            relation_mode: out.relation_mode,
        },
        out.diagnostics,
    )
}

/// Retrieves a Prisma schema without validating it.
/// You should only use this method when actually validating the schema is too expensive
/// computationally or in terms of bundle size (e.g., for `query-engine-wasm`).
pub fn parse_without_validation(file: SourceFile, connectors: ConnectorRegistry<'_>) -> ValidatedSchema {
    let mut diagnostics = Diagnostics::new();
    let db = ParserDatabase::new(file, &mut diagnostics);
    let (configuration, _) = validate_configuration(db.ast(), &mut diagnostics, connectors);
    let datasources = &configuration.datasources;
    let out = validate::parse_without_validation(db, datasources);

    ValidatedSchema {
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
) -> Result<(Configuration, Vec<DatamodelWarning>), diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let ast = schema_ast::parse_schema(schema, &mut diagnostics);
    let out = validate_configuration(&ast, &mut diagnostics, connectors);
    diagnostics.to_result().map(|_| out)
}

fn validate_configuration(
    schema_ast: &ast::SchemaAst,
    diagnostics: &mut Diagnostics,
    connectors: ConnectorRegistry<'_>,
) -> (Configuration, Vec<DatamodelWarning>) {
    let generators = generator_loader::load_generators_from_ast(schema_ast, diagnostics);
    let datasources = datasource_loader::load_datasources_from_ast(schema_ast, diagnostics, connectors);
    let warnings = diagnostics.warnings().to_owned();

    (
        Configuration {
            generators,
            datasources,
        },
        warnings,
    )
}
