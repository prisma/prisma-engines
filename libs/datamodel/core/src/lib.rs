//! This crate is responsible for parsing, rendering and formatting a Prisma Schema.
//! A Prisma Schema consists out of two parts:
//! 1. The `Datamodel` part refers to the model and enum definitions.
//! 2. The `Configuration` part refers to the generator and datasource definitions.
//!
//! The data structures are organized into 3 layers:
//! * The AST layer contains the data data structures representing the schema input.
//! * The model layer contains the data structures that are semantically rich and therefore engines can build upon.
//! * The JSON layer contains the data structures that represent the contract for the DMMF which is the API for client generators.
//!
//! The responsibilities of each top level module is as following:
//! * `common`: contains constants and generic helpers
//! * `error`: contains the error and result types
//! * `ast`: contains the data structures for the AST of a Prisma schema. And the parsing functions to turn an input string into an AST.
//! * `dml`: contains the models representing the Datamodel part of a Prisma schema
//! * `configuration`: contains the models representing the Datasources and Generators of a Prisma schema
//! * `transform`: contains the logic to turn an AST into models and vice versa
//! * `json`: contains the logic to turn models into their JSON/DMMF representation
//!
//! The flow between the layers is depicted in the following diagram.
//!<pre>
//!                ┌──────────────────┐
//!                │       json       │
//!                └────────▲─────────┘
//!                         │
//!                         │
//!      ┌──────────────────┐┌──────────────────┐
//!      │       dml        ││  configuration   │
//!      └──────────────────┘└──────────────────┘
//!                        │  ▲
//!┌─────────────────────┐ │  │ ┌─────────────────────┐
//!│transform::dml_to_ast│ │  │ │transform::ast_to_dml│
//!└─────────────────────┘ │  │ └─────────────────────┘
//!                        │  │
//!                        ▼  │
//!                ┌──────────────────┐
//!                │       ast        │
//!                └──────────────────┘
//!                        │  ▲
//!                        │  │
//!                        │  │
//!                        ▼  │
//!                 ┌──────────────────┐
//!                 │  schema string   │
//!                 └──────────────────┘
//!</pre>
//!
//! The usage dependencies between the main modules is depicted in the following diagram.
//! The modules `error` and `common` are not shown as any module may depend on them.
//!<pre>
//!                       ┌──────────────────┐
//!                       │    transform     │
//!                       └──────────────────┘
//!                                 │
//!                                 │ use
//!          ┌──────────────────────┼──────────────────────────┐
//!          │                      │                          │
//!          │                      │                          │
//!          ▼                      ▼                          ▼
//!┌──────────────────┐   ┌──────────────────┐       ┌──────────────────┐
//!│       ast        │   │       dml        │       │  configuration   │
//!└──────────────────┘   └──────────────────┘       └──────────────────┘
//!                                 ▲                          ▲
//!                                 │                          │
//!                                 ├──────────────────────────┘
//!                                 │ use
//!                                 │
//!                       ┌──────────────────┐
//!                       │       json       │
//!                       └──────────────────┘
//!</pre>
//!

#![deny(rust_2018_idioms, unsafe_code)]

pub mod ast;
pub mod common;
pub mod dml;
pub mod json;

mod configuration;
mod transform;

pub use crate::dml::*;
pub use ::dml::prisma_value;
pub use configuration::{Configuration, Datasource, Generator, StringFromEnvVar};
pub use datamodel_connector;
pub use diagnostics;
pub use parser_database;
pub use parser_database::is_reserved_type_name;
pub use schema_ast;

use crate::{ast::SchemaAst, common::preview_features::PreviewFeature};
use diagnostics::{Diagnostics, Validated};
use enumflags2::BitFlags;
use transform::{
    ast_to_dml::{validate, DatasourceLoader, GeneratorLoader},
    dml_to_ast::{self, GeneratorSerializer, LowerDmlToAst},
};

pub type ValidatedDatamodel = Validated<Datamodel>;
pub type ValidatedConfiguration = Validated<Configuration>;

/// Parse and validate the whole schema
pub fn parse_schema(schema_str: &str) -> Result<(Configuration, Datamodel), String> {
    parse_datamodel_internal(schema_str)
        .map_err(|err| err.to_pretty_string("schema.prisma", schema_str))
        .map(|v| v.subject)
}

pub struct ValidatedSchema<'a> {
    pub configuration: Configuration,
    pub db: parser_database::ParserDatabase<'a>,
    referential_integrity: datamodel_connector::ReferentialIntegrity,
}

impl<'a> ValidatedSchema<'a> {
    pub fn referential_integrity(&self) -> datamodel_connector::ReferentialIntegrity {
        self.referential_integrity
    }
}

/// Parse and validate the whole schema. This function's signature is obviously less than optimal,
/// let's work towards something simpler.
pub fn parse_schema_parserdb<'ast>(src: &'ast str, ast: &'ast ast::SchemaAst) -> Result<ValidatedSchema<'ast>, String> {
    let mut diagnostics = Diagnostics::new();
    let generators = GeneratorLoader::load_generators_from_ast(ast, &mut diagnostics);
    let preview_features = preview_features(&generators);
    let datasources = load_sources(ast, preview_features, &mut diagnostics);

    diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", src))?;

    let out = validate(src, ast, &datasources, preview_features, diagnostics);

    out.diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", src))?;

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

fn parse_datamodel_for_formatter(src: &str, ast: &SchemaAst) -> Result<(Datamodel, Vec<Datasource>), Diagnostics> {
    let mut diagnostics = diagnostics::Diagnostics::new();
    let datasources = load_sources(ast, Default::default(), &mut diagnostics);
    let (db, diagnostics) = parser_database::ParserDatabase::new(src, ast, diagnostics);
    diagnostics.to_result()?;
    let (connector, referential_integrity) = datasources
        .get(0)
        .map(|ds| (ds.active_connector, ds.referential_integrity()))
        .unwrap_or((&datamodel_connector::EmptyDatamodelConnector, Default::default()));

    let datamodel = transform::ast_to_dml::LiftAstToDml::new(&db, connector, referential_integrity).lift();
    Ok((datamodel, datasources))
}

fn parse_datamodel_internal(
    datamodel_string: &str,
) -> Result<Validated<(Configuration, Datamodel)>, diagnostics::Diagnostics> {
    let mut diagnostics = diagnostics::Diagnostics::new();
    let ast = ast::parse_schema(datamodel_string, &mut diagnostics);

    let generators = GeneratorLoader::load_generators_from_ast(&ast, &mut diagnostics);
    let preview_features = preview_features(&generators);
    let datasources = load_sources(&ast, preview_features, &mut diagnostics);

    diagnostics.to_result()?;

    let out = validate(datamodel_string, &ast, &datasources, preview_features, diagnostics);

    if !out.diagnostics.errors().is_empty() {
        return Err(out.diagnostics);
    }

    let datamodel = transform::ast_to_dml::LiftAstToDml::new(&out.db, out.connector, out.referential_integrity).lift();

    Ok(Validated {
        subject: (
            Configuration {
                generators,
                datasources,
            },
            datamodel,
        ),
        warnings: out.diagnostics.warnings().to_vec(),
    })
}

pub fn parse_schema_ast(datamodel_string: &str) -> Result<SchemaAst, diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let schema = ast::parse_schema(datamodel_string, &mut diagnostics);

    diagnostics.to_result()?;

    Ok(schema)
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(schema: &str) -> Result<ValidatedConfiguration, diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let ast = ast::parse_schema(schema, &mut diagnostics);

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
        warnings: diagnostics.warnings().to_owned(),
    })
}

fn load_sources(
    schema_ast: &SchemaAst,
    preview_features: BitFlags<PreviewFeature>,
    diagnostics: &mut Diagnostics,
) -> Vec<Datasource> {
    DatasourceLoader.load_datasources_from_ast(schema_ast, preview_features, diagnostics)
}

//
//  ************** RENDERING FUNCTIONS **************
//

/// Renders to a return string.
pub fn render_datamodel_to_string(datamodel: &dml::Datamodel, configuration: Option<&Configuration>) -> String {
    let mut writable_string = String::with_capacity(datamodel.models.len() * 20);
    render_datamodel_to(&mut writable_string, datamodel, configuration);
    writable_string
}

/// Renders an AST to a string.
pub fn render_schema_ast_to_string(schema: &SchemaAst) -> String {
    let mut writable_string = String::with_capacity(schema.tops.len() * 20);

    render_schema_ast_to(&mut writable_string, schema, 2);

    writable_string
}

/// Renders as a string into the stream.
pub fn render_datamodel_to(
    stream: &mut dyn std::fmt::Write,
    datamodel: &dml::Datamodel,
    configuration: Option<&Configuration>,
) {
    let datasource = configuration.and_then(|c| c.datasources.first());

    let preview_features = configuration
        .map(|c| c.preview_features())
        .unwrap_or_else(BitFlags::empty);

    let lowered = LowerDmlToAst::new(datasource, preview_features).lower(datamodel);

    render_schema_ast_to(stream, &lowered, 2);
}

/// Renders as a string into the stream.
pub fn render_datamodel_to_with_preview_flags(
    stream: &mut dyn std::fmt::Write,
    datamodel: &dml::Datamodel,
    datasource: Option<&Datasource>,
    flags: BitFlags<PreviewFeature>,
) {
    let lowered = LowerDmlToAst::new(datasource, flags).lower(datamodel);
    render_schema_ast_to(stream, &lowered, 2);
}

/// Renders a datamodel, sources and generators to a string.
pub fn render_datamodel_and_config_to_string(
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) -> String {
    let mut writable_string = String::with_capacity(datamodel.models.len() * 20);

    render_datamodel_and_config_to(&mut writable_string, datamodel, config);

    writable_string
}

/// Renders a datamodel, generators and sources to a stream as a string.
fn render_datamodel_and_config_to(
    stream: &mut dyn std::fmt::Write,
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) {
    let mut lowered = LowerDmlToAst::new(config.datasources.first(), config.preview_features()).lower(datamodel);

    dml_to_ast::add_sources_to_ast(config, &mut lowered);
    GeneratorSerializer::add_generators_to_ast(&config.generators, &mut lowered);

    render_schema_ast_to(stream, &lowered, 2);
}

/// Renders as a string into the stream.
fn render_schema_ast_to(stream: &mut dyn std::fmt::Write, schema: &ast::SchemaAst, ident_width: usize) {
    let mut renderer = ast::Renderer::new(stream, ident_width);
    renderer.render(schema);
}

fn preview_features(generators: &[Generator]) -> BitFlags<PreviewFeature> {
    generators.iter().map(|gen| gen.preview_features()).collect()
}
