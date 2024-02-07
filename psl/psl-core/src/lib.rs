#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms, unsafe_code)]
#![feature(trait_upcasting)]
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerdeValidatedSchema {
    pub db: parser_database::ParserDatabase,
    pub preview_features: enumflags2::BitFlags<PreviewFeature>,
    pub relation_mode: datamodel_connector::RelationMode,
    pub provider: String,
}

impl SerdeValidatedSchema {
    pub fn into_schema_for_qe(self, connectors: &ValidatedConnectorRegistry<'_>) -> ValidatedSchemaForQE {
        let active_connector = connectors
            .iter()
            .find(|c| c.is_provider(self.provider.as_str()))
            .unwrap();

        ValidatedSchemaForQE {
            db: self.db,
            preview_features: self.preview_features,
            relation_mode: self.relation_mode,
            connector: *active_connector,
        }
    }
}

/// `SchemaForQE` is the `query-engine`-specific specific variant of `ValidatedSchema`.
pub struct ValidatedSchemaForQE {
    pub db: parser_database::ParserDatabase,
    pub preview_features: enumflags2::BitFlags<PreviewFeature>,
    pub relation_mode: datamodel_connector::RelationMode,
    pub connector: &'static dyn datamodel_connector::ValidatedConnector,
}

impl ValidatedSchemaForQE {
    pub fn preview_features(&self) -> enumflags2::BitFlags<PreviewFeature> {
        self.preview_features
    }

    pub fn relation_mode(&self) -> datamodel_connector::RelationMode {
        self.relation_mode
    }
}

impl From<ValidatedSchema> for SerdeValidatedSchema {
    fn from(schema: ValidatedSchema) -> Self {
        Self {
            db: schema.db,
            preview_features: schema.configuration.preview_features(),
            relation_mode: schema.relation_mode,
            provider: schema.connector.provider_name().to_owned(),
        }
    }
}

impl From<ValidatedSchema> for ValidatedSchemaForQE {
    fn from(schema: ValidatedSchema) -> Self {
        Self {
            db: schema.db,
            preview_features: schema.configuration.preview_features(),
            relation_mode: schema.relation_mode,
            connector: schema.connector,
        }
    }
}

impl std::fmt::Debug for ValidatedSchemaForQE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<Prisma schema>")
    }
}

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

/// Given a textual `.prisma` file, it:
/// - parses it
/// - validates it
pub fn serialize_to_bytes(file: SourceFile, connectors: ConnectorRegistry<'_>) -> Result<Vec<u8>, String> {
    let (validated_schema, mut diagnostics) = validate(file, connectors);

    if let Err(err) = diagnostics.to_result() {
        return Err(err.to_pretty_string("schema.prisma", validated_schema.db.source()));
    }

    let serde_schema = SerdeValidatedSchema::from(validated_schema);

    postcard::to_allocvec(&serde_schema).map_err(|e| format!("[serialize]: {}", e.to_string()))
}

pub fn deserialize_from_bytes(
    schema_as_binary: &[u8],
    connectors: &ValidatedConnectorRegistry<'_>,
) -> Result<ValidatedSchemaForQE, String> {
    let serde_schema: SerdeValidatedSchema =
        postcard::from_bytes(schema_as_binary).map_err(|e| format!("[deserialize] {}", e.to_string()))?;

    Ok(serde_schema.into_schema_for_qe(connectors))
}

/// Retrieves a Prisma schema without validating it.
/// You should only use this method when actually validating the schema is too expensive
/// computationally or in terms of bundle size (e.g., for `query-engine-wasm`).
/// Note: this should be deprecated by this PR.
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
