// Load macros first - Global Macros used for parsing.
// Macro to match all children in a parse tree
#[macro_use]
macro_rules! match_children (
    ($token:ident, $current:ident, $($pattern:pat => $result:expr),*) => (
        // Explicit clone, as into_inner consumes the pair.
        // We only need a reference to the pair later for logging.
        for $current in $token.clone().into_inner() {
            match $current.as_rule() {
                Rule::WHITESPACE => { },
                Rule::COMMENT => { },
                Rule::BLOCK_OPEN => { },
                Rule::BLOCK_CLOSE => { },
                $(
                    $pattern => $result
                ),*
            }
        }
    );
);

// Macro to match the first child in a parse tree
#[macro_use]
macro_rules! match_first (
    ($token:ident, $current:ident, $($pattern:pat => $result:expr),*) => ( {
            // Explicit clone, as into_inner consumes the pair.
        // We only need a reference to the pair later for logging.
            let $current = $token.clone()
                .into_inner()
                .filter(|rule|
                    rule.as_rule() != Rule::BLOCK_CLOSE &&
                    rule.as_rule() != Rule::BLOCK_OPEN &&
                    rule.as_rule() != Rule::WHITESPACE &&
                    rule.as_rule() != Rule::COMMENT)
                .next().unwrap();
            match $current.as_rule() {
                $(
                    $pattern => $result
                ),*
            }
        }
    );
);

extern crate pest; // Pest grammar generation on compile time.
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate failure; // Failure enum display derivation

pub mod ast;
pub mod common;
pub mod configuration;
pub mod dml;
pub mod error;
pub mod json;
pub mod validator;

pub use configuration::*;
pub use dml::*;

use crate::ast::SchemaAst;
use std::io::Write;
use validator::ValidationPipeline;

/// Parses and validates a datamodel string, using core attributes only.
pub fn parse_datamodel(datamodel_string: &str) -> Result<Datamodel, error::ErrorCollection> {
    parse_datamodel_with_sources(datamodel_string, vec![])
}

/// Validates a [Schema AST](/ast/struct.SchemaAst.html) and returns its
/// [Datamodel](/struct.Datamodel.html).
pub fn lift_ast(ast: &ast::SchemaAst) -> Result<Datamodel, error::ErrorCollection> {
    let mut errors = error::ErrorCollection::new();
    let sources = load_sources(ast, vec![])?;
    let validator = ValidationPipeline::with_sources(&sources);

    match validator.validate(&ast) {
        Ok(src) => Ok(src),
        Err(mut err) => {
            errors.append(&mut err);
            Err(errors)
        }
    }
}

/// Parses and validates a datamodel string, using core attributes only.
/// In case of an error, a pretty, colorful string is returned.
pub fn parse_datamodel_or_pretty_error(datamodel_string: &str, file_name: &str) -> Result<Datamodel, String> {
    match parse_datamodel_with_sources(datamodel_string, vec![]) {
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

/// Parses and validates a datamodel string, using core attributes and the given sources.
/// If source loading failes, validation continues, but an error is returned.
pub fn parse_datamodel_with_sources(
    datamodel_string: &str,
    source_definitions: Vec<Box<dyn configuration::SourceDefinition>>,
) -> Result<Datamodel, error::ErrorCollection> {
    let ast = ast::parser::parse(datamodel_string)?;

    let mut errors = error::ErrorCollection::new();

    let sources = match load_sources(&ast, source_definitions) {
        Ok(src) => src,
        Err(mut err) => {
            errors.append(&mut err);
            Vec::new()
        }
    };
    let validator = ValidationPipeline::with_sources(&sources);

    match validator.validate(&ast) {
        Ok(src) => Ok(src),
        Err(mut err) => {
            errors.append(&mut err);
            Err(errors)
        }
    }
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(datamodel_string: &str) -> Result<Configuration, error::ErrorCollection> {
    parse_configuration_with_sources(datamodel_string, vec![])
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions and extra given ones.
pub fn parse_configuration_with_sources(
    datamodel_string: &str,
    source_definitions: Vec<Box<dyn configuration::SourceDefinition>>,
) -> Result<Configuration, error::ErrorCollection> {
    let ast = ast::parser::parse(datamodel_string)?;
    let datasources = load_sources(&ast, source_definitions)?;
    let generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    Ok(Configuration {
        datasources,
        generators,
    })
}

fn load_sources(
    schema_ast: &SchemaAst,
    source_definitions: Vec<Box<dyn configuration::SourceDefinition>>,
) -> Result<Vec<Box<dyn Source>>, error::ErrorCollection> {
    let mut source_loader = SourceLoader::new();
    for source in get_builtin_sources() {
        source_loader.add_source_definition(source);
    }
    for source in source_definitions {
        source_loader.add_source_definition(source);
    }

    source_loader.load(&schema_ast)
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
    let lowered = validator::LowerDmlToAst::new().lower(datamodel)?;
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
pub fn render_datamodel_and_config_to(
    stream: &mut dyn std::io::Write,
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) -> Result<(), error::ErrorCollection> {
    let mut lowered = validator::LowerDmlToAst::new().lower(datamodel)?;
    SourceSerializer::add_sources_to_ast(&config.datasources, &mut lowered);
    GeneratorLoader::add_generators_to_ast(&config.generators, &mut lowered);
    render_schema_ast_to(stream, &lowered, 2);
    Ok(())
}

/// Renders as a string into the stream.
fn render_schema_ast_to(stream: &mut dyn std::io::Write, schema: &ast::SchemaAst, ident_width: usize) {
    let mut renderer = ast::renderer::Renderer::new(stream, ident_width);
    renderer.render(schema);
}

// Convenience Helpers
pub fn get_builtin_sources() -> Vec<Box<dyn SourceDefinition>> {
    vec![
        Box::new(configuration::builtin::MySqlSourceDefinition::new()),
        Box::new(configuration::builtin::PostgresSourceDefinition::new()),
        Box::new(configuration::builtin::SqliteSourceDefinition::new()),
    ]
}
