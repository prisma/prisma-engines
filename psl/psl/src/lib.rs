#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

pub use builtin_psl_connectors as builtin_connectors;
pub use psl_core::{
    datamodel_connector,
    diagnostics::{self, Diagnostics},
    is_reserved_type_name,
    mcf::config_to_mcf_json_value as get_config,
    mcf::{generators_to_json, render_sources_to_json}, // for tests
    parser_database::{self, SourceFile},
    reformat,
    schema_ast,
    Configuration,
    Datasource,
    DatasourceConnectorData,
    Generator,
    PreviewFeature,
    PreviewFeatures,
    StringFromEnvVar,
    ValidatedSchema,
    ALL_PREVIEW_FEATURES,
};

/// The implementation of the CLI getConfig() utility and its JSON format.
pub mod get_config {
    pub use psl_core::mcf::{config_to_mcf_json_value as get_config, *};
}

/// Parses and validate a schema, but skip analyzing everything except datasource and generator
/// blocks.
pub fn parse_configuration(schema: &str) -> Result<Configuration, Diagnostics> {
    psl_core::parse_configuration(schema, builtin_connectors::BUILTIN_CONNECTORS)
}

/// Parse and analyze a Prisma schema.
pub fn parse_schema(file: impl Into<SourceFile>) -> Result<ValidatedSchema, String> {
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
    psl_core::validate(file, builtin_connectors::BUILTIN_CONNECTORS)
}
