#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

pub use psl_core::builtin_connectors;
use psl_core::parser_database::Files;
pub use psl_core::{
    builtin_connectors::{can_have_capability, can_support_relation_load_strategy, has_capability},
    datamodel_connector,
    diagnostics::{self, Diagnostics},
    is_reserved_type_name,
    mcf::config_to_mcf_json_value as get_config,
    mcf::{generators_to_json, render_sources_to_json}, // for tests
    parser_database::{self, SourceFile},
    reachable_only_with_capability,
    reformat,
    reformat_multiple,
    reformat_validated_schema_into_single,
    schema_ast,
    set_config_dir,
    Configuration,
    ConnectorRegistry,
    Datasource,
    DatasourceConnectorData,
    Generator,
    GeneratorConfigValue,
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

/// Parses and validates Prisma schemas, but skip analyzing everything except datasource and generator
/// blocks.
pub fn parse_configuration_multi_file(
    files: &[(String, SourceFile)],
) -> Result<(Files, Configuration), (Files, Diagnostics)> {
    psl_core::parse_configuration_multi_file(files, builtin_connectors::BUILTIN_CONNECTORS)
}

/// Parses and validates Prisma schemas, but skip analyzing everything except datasource and generator
/// blocks. It never fails, but when the returned `Diagnostics` contains errors, it implies that the
/// `Configuration` content is partial.
/// Consumers may then decide  whether to convert `Diagnostics` into an error.
pub fn error_tolerant_parse_configuration(files: &[(String, SourceFile)]) -> (Files, Configuration, Diagnostics) {
    psl_core::error_tolerant_parse_configuration(files, builtin_connectors::BUILTIN_CONNECTORS)
}

/// Parse and analyze a Prisma schema.
pub fn parse_schema(file: impl Into<SourceFile>) -> Result<ValidatedSchema, String> {
    let mut schema = validate(file.into());
    schema
        .diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", schema.db.source_assert_single()))?;
    Ok(schema)
}

/// Parse and analyze a Prisma schema.
pub fn parse_schema_multi(files: &[(String, SourceFile)]) -> Result<ValidatedSchema, String> {
    let mut schema = validate_multi_file(files);

    schema
        .diagnostics
        .to_result()
        .map_err(|err| schema.db.render_diagnostics(&err))?;

    Ok(schema)
}

/// The most general API for dealing with Prisma schemas. It accumulates what analysis and
/// validation information it can, and returns it along with any error and warning diagnostics.
pub fn validate(file: SourceFile) -> ValidatedSchema {
    psl_core::validate(file, builtin_connectors::BUILTIN_CONNECTORS)
}

/// Parse a Prisma schema, but skip validations.
pub fn parse_without_validation(file: SourceFile, connector_registry: ConnectorRegistry<'_>) -> ValidatedSchema {
    psl_core::parse_without_validation(file, connector_registry)
}
/// The most general API for dealing with Prisma schemas. It accumulates what analysis and
/// validation information it can, and returns it along with any error and warning diagnostics.
pub fn validate_multi_file(files: &[(String, SourceFile)]) -> ValidatedSchema {
    psl_core::validate_multi_file(files, builtin_connectors::BUILTIN_CONNECTORS)
}
