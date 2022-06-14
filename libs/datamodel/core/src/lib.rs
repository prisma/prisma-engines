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
//! * `mcf`: contains the logic to turn generators and datasources into their JSON representation
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

#![deny(rust_2018_idioms, unsafe_code)]

pub mod common;
pub mod dml;

/// `mcf`: Turns a collection of `configuration::Datasource` and `configuration::Generator` into a JSON representation.
pub mod mcf;

mod configuration;
mod reformat;
mod transform;

pub use crate::{
    configuration::{Configuration, Datasource, Generator, StringFromEnvVar},
    reformat::reformat,
};
pub use datamodel_connector;
pub use diagnostics;
pub use parser_database;
pub use parser_database::is_reserved_type_name;
pub use schema_ast::{self, ast};

use crate::common::preview_features::PreviewFeature;
use diagnostics::Diagnostics;
use enumflags2::BitFlags;
use parser_database::ParserDatabase;
use transform::{
    ast_to_dml::{validate, DatasourceLoader, GeneratorLoader},
    dml_to_ast::{self, lower, GeneratorSerializer, LowerParams},
};

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

/// Parse and validate the whole schema.
pub fn parse_schema_parserdb(src: &str) -> Result<ValidatedSchema, String> {
    let mut diagnostics = Diagnostics::new();
    diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", src))?;

    let ast = schema_ast::parser::parse_schema(src, &mut diagnostics);
    let generators = GeneratorLoader::load_generators_from_ast(&ast, &mut diagnostics);
    let preview_features = preview_features(&generators);
    let datasources = load_sources(&ast, preview_features, &mut diagnostics);

    diagnostics
        .to_result()
        .map_err(|err| err.to_pretty_string("schema.prisma", src))?;

    let out = validate(ast, &datasources, preview_features, diagnostics);

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

fn parse_datamodel_internal(
    datamodel_string: &str,
) -> Result<Validated<(Configuration, dml::Datamodel)>, diagnostics::Diagnostics> {
    let mut diagnostics = diagnostics::Diagnostics::new();
    let ast = schema_ast::parser::parse_schema(datamodel_string, &mut diagnostics);

    let generators = GeneratorLoader::load_generators_from_ast(&ast, &mut diagnostics);
    let preview_features = preview_features(&generators);
    let datasources = load_sources(&ast, preview_features, &mut diagnostics);

    diagnostics.to_result()?;

    let out = validate(ast, &datasources, preview_features, diagnostics);

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

pub fn parse_schema_ast(datamodel_string: &str) -> Result<ast::SchemaAst, diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let schema = schema_ast::parser::parse_schema(datamodel_string, &mut diagnostics);

    diagnostics.to_result()?;

    Ok(schema)
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(schema: &str) -> Result<ValidatedConfiguration, diagnostics::Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let ast = schema_ast::parser::parse_schema(schema, &mut diagnostics);

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
    schema_ast: &ast::SchemaAst,
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
pub fn render_schema_ast_to_string(schema: &ast::SchemaAst) -> String {
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
    let lowered = lower(LowerParams { datasource, datamodel });

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
    let mut lowered = lower(LowerParams {
        datasource: config.datasources.first(),
        datamodel,
    });

    dml_to_ast::add_sources_to_ast(config, &mut lowered);
    GeneratorSerializer::add_generators_to_ast(&config.generators, &mut lowered);

    render_schema_ast_to(stream, &lowered, 2);
}

/// Renders as a string into the stream.
fn render_schema_ast_to(stream: &mut dyn std::fmt::Write, schema: &ast::SchemaAst, ident_width: usize) {
    let mut renderer = schema_ast::renderer::Renderer::new(stream, ident_width);
    renderer.render(schema);
}

fn preview_features(generators: &[Generator]) -> BitFlags<PreviewFeature> {
    generators.iter().map(|gen| gen.preview_features()).collect()
}
