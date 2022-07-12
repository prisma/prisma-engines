//! PSL — Prisma Schema Language
//!
//! This crate is responsible for parsing, validating, formatting and rendering a PSL documents.
//!
//! A Prisma Schema consists out of two parts:
//! 1. The `Datamodel` part refers to the model and enum definitions.
//! 2. The `Configuration` part refers to the generator and datasource definitions.
//!

#![deny(rust_2018_idioms, unsafe_code)]

pub mod common;

/// `mcf`: Turns a collection of `configuration::Datasource` and `configuration::Generator` into a JSON representation.
pub mod mcf;

mod configuration;
mod lift;
mod reformat;
mod render;
mod validate;

pub use crate::{
    configuration::{Configuration, Datasource, Generator, StringFromEnvVar},
    reformat::reformat,
};
pub use datamodel_connector;
pub use diagnostics;
pub use dml;
pub use parser_database;
pub use parser_database::is_reserved_type_name;

use self::{
    render::RenderParams,
    validate::{validate, DatasourceLoader, GeneratorLoader},
};
use crate::common::preview_features::PreviewFeature;
use diagnostics::Diagnostics;
use enumflags2::BitFlags;
use parser_database::{ast, ParserDatabase, SourceFile};
use std::sync::Arc;

pub mod builtin_connectors {
    pub use mongodb_datamodel_connector::*;
    pub use sql_datamodel_connector::*;
}

#[derive(Debug)]
pub struct Validated<T> {
    pub subject: T,
    pub warnings: Vec<diagnostics::DatamodelWarning>,
}

pub type ValidatedDatamodel = Validated<dml::Datamodel>;
pub type ValidatedConfiguration = Validated<Configuration>;

/// Parse and validate the whole schema
pub fn parse_schema(schema_str: &str) -> Result<(Configuration, dml::Datamodel), String> {
    parse_datamodel_internal(schema_str)
        .map_err(|err| err.to_pretty_string("schema.prisma", schema_str))
        .map(|v| v.subject)
}

pub struct ValidatedSchema {
    pub configuration: Configuration,
    pub db: parser_database::ParserDatabase,
    referential_integrity: datamodel_connector::ReferentialIntegrity,
}

impl ValidatedSchema {
    pub fn referential_integrity(&self) -> datamodel_connector::ReferentialIntegrity {
        self.referential_integrity
    }
}

pub fn parse_schema_parserdb(file: impl Into<SourceFile>) -> Result<ValidatedSchema, String> {
    let file = file.into();

    let mut diagnostics = Diagnostics::new();
    let db = ParserDatabase::new(file.clone(), &mut diagnostics);

    diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", file.as_str()))?;

    let generators = GeneratorLoader::load_generators_from_ast(db.ast(), &mut diagnostics);
    let preview_features = preview_features(&generators);
    let datasources = load_sources(db.ast(), preview_features, &mut diagnostics);

    let mut out = validate(db, &datasources, preview_features, diagnostics);
    out.diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", file.as_str()))?;

    Ok(ValidatedSchema {
        configuration: Configuration {
            generators,
            datasources,
        },
        db: out.db,
        referential_integrity: out.referential_integrity,
    })
}

/// Parses and validates a datamodel string, using core attributes only.
pub fn parse_datamodel(datamodel_string: &str) -> Result<ValidatedDatamodel, diagnostics::Diagnostics> {
    parse_datamodel_internal(datamodel_string).map(|validated| Validated {
        subject: validated.subject.1,
        warnings: validated.warnings,
    })
}

fn parse_datamodel_internal(
    datamodel_string: &str,
) -> Result<Validated<(Configuration, dml::Datamodel)>, diagnostics::Diagnostics> {
    let file = SourceFile::new_allocated(Arc::from(datamodel_string.to_owned().into_boxed_str()));

    let mut diagnostics = diagnostics::Diagnostics::new();
    let db = ParserDatabase::new(file, &mut diagnostics);

    diagnostics.to_result()?;

    let generators = GeneratorLoader::load_generators_from_ast(db.ast(), &mut diagnostics);
    let preview_features = preview_features(&generators);
    let datasources = load_sources(db.ast(), preview_features, &mut diagnostics);

    diagnostics.to_result()?;

    let out = validate(db, &datasources, preview_features, diagnostics);

    if !out.diagnostics.errors().is_empty() {
        return Err(out.diagnostics);
    }

    let datamodel = lift::LiftAstToDml::new(&out.db, out.connector, out.referential_integrity).lift();

    Ok(Validated {
        subject: (
            Configuration {
                generators,
                datasources,
            },
            datamodel,
        ),
        warnings: out.diagnostics.into_warnings(),
    })
}

pub fn parse_schema_ast(datamodel_string: &str) -> Result<ast::SchemaAst, diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let schema = schema_ast::parse_schema(datamodel_string, &mut diagnostics);

    diagnostics.to_result()?;

    Ok(schema)
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(schema: &str) -> Result<ValidatedConfiguration, diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let ast = schema_ast::parse_schema(schema, &mut diagnostics);

    diagnostics.to_result()?;

    let generators = GeneratorLoader::load_generators_from_ast(&ast, &mut diagnostics);
    let preview_features = preview_features(&generators);
    let datasources = load_sources(&ast, preview_features, &mut diagnostics);

    diagnostics.to_result()?;

    Ok(ValidatedConfiguration {
        subject: Configuration {
            generators,
            datasources,
        },
        warnings: diagnostics.into_warnings(),
    })
}

fn load_sources(
    schema_ast: &ast::SchemaAst,
    preview_features: BitFlags<PreviewFeature>,
    diagnostics: &mut Diagnostics,
) -> Vec<Datasource> {
    DatasourceLoader.load_datasources_from_ast(schema_ast, preview_features, diagnostics)
}

//
//  ************** RENDERING FUNCTIONS **************
//

/// Renders the datamodel _without configuration blocks_.
pub fn render_datamodel_to_string(datamodel: &dml::Datamodel, configuration: Option<&Configuration>) -> String {
    let datasource = configuration.and_then(|c| c.datasources.first());
    let mut out = String::new();
    render::render_datamodel(RenderParams { datasource, datamodel }, &mut out);
    reformat(&out, DEFAULT_INDENT_WIDTH).expect("Internal error: failed to reformat introspected schema")
}

/// Renders a datamodel, sources and generators.
pub fn render_datamodel_and_config_to_string(
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) -> String {
    let mut out = String::new();
    let datasource = config.datasources.first();
    render::render_configuration(config, &mut out);
    render::render_datamodel(RenderParams { datasource, datamodel }, &mut out);
    reformat(&out, DEFAULT_INDENT_WIDTH).expect("Internal error: failed to reformat introspected schema")
}

fn preview_features(generators: &[Generator]) -> BitFlags<PreviewFeature> {
    generators.iter().map(|gen| gen.preview_features()).collect()
}

const DEFAULT_INDENT_WIDTH: usize = 2;
