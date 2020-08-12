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
extern crate pest; // Pest grammar generation on compile time.
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate tracing;

pub mod ast;
pub mod common;
pub mod configuration;
pub mod dml;
pub mod error;
pub mod json;
pub mod transform;
pub mod walkers;

pub use configuration::*;
pub use dml::*;

use crate::ast::SchemaAst;
use std::io::Write;
use transform::{
    ast_to_dml::{DatasourceLoader, GeneratorLoader, ValidationPipeline},
    dml_to_ast::{DatasourceSerializer, GeneratorSerializer, LowerDmlToAst},
};

/// Parses and validates a datamodel string, using core attributes only.
pub fn parse_datamodel(datamodel_string: &str) -> Result<Datamodel, error::ErrorCollection> {
    parse_datamodel_internal(datamodel_string, false)
}

pub fn parse_datamodel_and_ignore_datasource_urls(datamodel_string: &str) -> Result<Datamodel, error::ErrorCollection> {
    parse_datamodel_internal(datamodel_string, true)
}

/// Parses and validates a datamodel string, using core attributes only.
/// In case of an error, a pretty, colorful string is returned.
pub fn parse_datamodel_or_pretty_error(datamodel_string: &str, file_name: &str) -> Result<Datamodel, String> {
    match parse_datamodel_internal(datamodel_string, false) {
        Ok(dml) => Ok(dml),
        Err(errs) => {
            let mut buffer = std::io::Cursor::new(Vec::<u8>::new());

            for error in errs.to_iter() {
                writeln!(&mut buffer).expect("Failed to render error.");
                error
                    .pretty_print(&mut buffer, file_name, datamodel_string)
                    .expect("Failed to render error.");
            }

            Err(String::from_utf8(buffer.into_inner()).expect("Failed to convert error buffer."))
        }
    }
}

fn parse_datamodel_internal(
    datamodel_string: &str,
    ignore_datasource_urls: bool,
) -> Result<Datamodel, error::ErrorCollection> {
    let ast = ast::parser::parse_schema(datamodel_string)?;
    let sources = load_sources(&ast, ignore_datasource_urls, vec![])?;
    let validator = ValidationPipeline::new(&sources);

    validator.validate(&ast)
}

/// Validates a [Schema AST](/ast/struct.SchemaAst.html) and returns its
/// [Datamodel](/struct.Datamodel.html).
pub fn lift_ast_to_datamodel(ast: &ast::SchemaAst) -> Result<Datamodel, error::ErrorCollection> {
    let mut errors = error::ErrorCollection::new();
    // we are not interested in the sources in this case. Hence we can ignore the datasource urls.
    let sources = load_sources(ast, true, vec![])?;
    let validator = ValidationPipeline::new(&sources);

    match validator.validate(&ast) {
        Ok(src) => Ok(src),
        Err(mut err) => {
            errors.append(&mut err);
            Err(errors)
        }
    }
}

pub fn parse_schema_ast(datamodel_string: &str) -> Result<SchemaAst, error::ErrorCollection> {
    ast::parser::parse_schema(datamodel_string)
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(datamodel_string: &str) -> Result<Configuration, error::ErrorCollection> {
    let ast = ast::parser::parse_schema(datamodel_string)?;
    let datasources = load_sources(&ast, false, vec![])?;
    let generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    Ok(Configuration {
        datasources,
        generators,
    })
}

/// - `datasource_url_overrides`: the tuples consist of datasource name and url
pub fn parse_configuration_with_url_overrides(
    schema: &str,
    datasource_url_overrides: Vec<(String, String)>,
) -> Result<Configuration, error::ErrorCollection> {
    let ast = ast::parser::parse_schema(schema)?;
    let datasources = load_sources(&ast, false, datasource_url_overrides)?;
    let generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    Ok(Configuration {
        datasources,
        generators,
    })
}

pub fn parse_configuration_and_ignore_datasource_urls(
    datamodel_string: &str,
) -> Result<Configuration, error::ErrorCollection> {
    let ast = ast::parser::parse_schema(datamodel_string)?;
    let datasources = load_sources(&ast, true, vec![])?;
    let generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    Ok(Configuration {
        datasources,
        generators,
    })
}

fn load_sources(
    schema_ast: &SchemaAst,
    ignore_datasource_urls: bool,
    datasource_url_overrides: Vec<(String, String)>,
) -> Result<Vec<Datasource>, error::ErrorCollection> {
    let source_loader = DatasourceLoader::new();
    source_loader.load_datasources_from_ast(&schema_ast, ignore_datasource_urls, datasource_url_overrides)
}

//
//  ************** RENDERING FUNCTIONS **************
//

/// Renders to a return string.
pub fn render_datamodel_to_string(datamodel: &dml::Datamodel) -> Result<String, error::ErrorCollection> {
    let mut writable_string = common::WritableString::new();
    render_datamodel_to(&mut writable_string, datamodel)?;
    Ok(writable_string.into())
}

/// Renders an AST to a string.
pub fn render_schema_ast_to_string(schema: &SchemaAst) -> Result<String, error::ErrorCollection> {
    let mut writable_string = common::WritableString::new();
    render_schema_ast_to(&mut writable_string, &schema, 2);
    Ok(writable_string.into())
}

/// Renders as a string into the stream.
pub fn render_datamodel_to(
    stream: &mut dyn std::io::Write,
    datamodel: &dml::Datamodel,
) -> Result<(), error::ErrorCollection> {
    let lowered = LowerDmlToAst::new().lower(datamodel)?;
    render_schema_ast_to(stream, &lowered, 2);
    Ok(())
}

/// Renders a datamodel, sources and generators to a string.
pub fn render_datamodel_and_config_to_string(
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) -> Result<String, error::ErrorCollection> {
    let mut writable_string = common::WritableString::new();
    render_datamodel_and_config_to(&mut writable_string, datamodel, config)?;
    Ok(writable_string.into())
}

/// Renders a datamodel, generators and sources to a stream as a string.
fn render_datamodel_and_config_to(
    stream: &mut dyn std::io::Write,
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) -> Result<(), error::ErrorCollection> {
    let mut lowered = LowerDmlToAst::new().lower(datamodel)?;

    DatasourceSerializer::add_sources_to_ast(config.datasources.as_slice(), &mut lowered);
    GeneratorSerializer::add_generators_to_ast(&config.generators, &mut lowered);

    render_schema_ast_to(stream, &lowered, 2);

    Ok(())
}

/// Renders as a string into the stream.
pub(crate) fn render_schema_ast_to(stream: &mut dyn std::io::Write, schema: &ast::SchemaAst, ident_width: usize) {
    let mut renderer = ast::renderer::Renderer::new(stream, ident_width);
    renderer.render(schema);
}
