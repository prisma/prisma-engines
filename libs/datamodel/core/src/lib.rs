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
                Rule::BLOCK_OPEN => { },
                Rule::BLOCK_CLOSE => { },
                Rule::NEWLINE => { },
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
                    rule.as_rule() != Rule::NEWLINE
                )
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
#[macro_use]
extern crate tracing;

pub mod ast;
pub mod common;
pub mod configuration;
pub mod dml;
pub mod error;
pub mod json;
pub mod validator;
pub mod walkers;

pub use common::DefaultNames;
pub use configuration::*;
pub use dml::*;

use crate::ast::SchemaAst;
use std::io::Write;
use validator::ValidationPipeline;

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
    let validator = ValidationPipeline::with_sources(&sources);

    validator.validate(&ast)
}

/// Validates a [Schema AST](/ast/struct.SchemaAst.html) and returns its
/// [Datamodel](/struct.Datamodel.html).
pub fn lift_ast_to_datamodel(ast: &ast::SchemaAst) -> Result<Datamodel, error::ErrorCollection> {
    let mut errors = error::ErrorCollection::new();
    // we are not interested in the sources in this case. Hence we can ignore the datasource urls.
    let sources = load_sources(ast, true, vec![])?;
    let validator = ValidationPipeline::with_sources(&sources);

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
    let source_loader = SourceLoader::new();
    source_loader.load_sources(&schema_ast, ignore_datasource_urls, datasource_url_overrides)
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

    SourceSerializer::add_sources_to_ast(config.datasources.as_slice(), &mut lowered);
    GeneratorLoader::add_generators_to_ast(&config.generators, &mut lowered);

    render_schema_ast_to(stream, &lowered, 2);

    Ok(())
}

/// Renders as a string into the stream.
pub(crate) fn render_schema_ast_to(stream: &mut dyn std::io::Write, schema: &ast::SchemaAst, ident_width: usize) {
    let mut renderer = ast::renderer::Renderer::new(stream, ident_width);
    renderer.render(schema);
}
