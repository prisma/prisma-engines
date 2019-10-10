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

// Lib exports.

pub mod ast;
pub mod dml;
pub use dml::*;
pub mod common;
pub use crate::common::FromStrAndSpan;
pub use common::argument::Arguments;
pub mod configuration;
pub mod dmmf;
pub mod error;
pub use common::functions::FunctionalEvaluator;
pub use configuration::*;
pub use validator::directive::DirectiveValidator;

mod validator;
pub use validator::ValidationPipeline;

use std::io::Write;

// Pest grammar generation on compile time.
extern crate pest;
#[macro_use]
extern crate pest_derive;

// Failure enum display derivation
#[macro_use]
extern crate failure;

// Convenience Helpers
pub fn get_builtin_sources() -> Vec<Box<dyn SourceDefinition>> {
    vec![
        Box::new(configuration::builtin::MySqlSourceDefinition::new()),
        Box::new(configuration::builtin::PostgresSourceDefinition::new()),
        Box::new(configuration::builtin::SqliteSourceDefinition::new()),
    ]
}

/// Parses and validates a datamodel string, using core attributes and the given plugins.
/// If plugin loading failes, validation continues, but an error is returned.
pub fn parse_with_plugins(
    datamodel_string: &str,
    source_definitions: Vec<Box<dyn configuration::SourceDefinition>>,
) -> Result<Datamodel, error::ErrorCollection> {
    let ast = ast::parser::parse(datamodel_string)?;
    let mut source_loader = SourceLoader::new();
    for source in get_builtin_sources() {
        source_loader.add_source_definition(source);
    }
    for source in source_definitions {
        source_loader.add_source_definition(source);
    }

    let mut errors = error::ErrorCollection::new();

    let sources = match source_loader.load(&ast) {
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

/// Loads all configuration blocks from a datamodel using the given source definitions.
pub fn load_configuration_with_plugins(
    datamodel_string: &str,
    source_definitions: Vec<Box<dyn configuration::SourceDefinition>>,
) -> Result<Configuration, error::ErrorCollection> {
    let ast = ast::parser::parse(datamodel_string)?;

    let mut source_loader = SourceLoader::new();
    for source in get_builtin_sources() {
        source_loader.add_source_definition(source);
    }
    for source in source_definitions {
        source_loader.add_source_definition(source);
    }

    let datasources = source_loader.load(&ast)?;

    let generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    Ok(Configuration {
        datasources,
        generators,
    })
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn load_configuration(datamodel_string: &str) -> Result<Configuration, error::ErrorCollection> {
    load_configuration_with_plugins(datamodel_string, vec![])
}

/// Parses and validates a datamodel string, using core attributes only.
pub fn parse(datamodel_string: &str) -> Result<Datamodel, error::ErrorCollection> {
    parse_with_plugins(datamodel_string, vec![])
}

/// Parses and validates a datamodel string, using core attributes only.
/// In case of an error, a pretty, colorful string is returned.
pub fn parse_with_formatted_error(datamodel_string: &str, file_name: &str) -> Result<Datamodel, String> {
    match parse_with_plugins(datamodel_string, vec![]) {
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

/// Parses a datamodel string to an AST. For internal use only.
pub fn parse_to_ast(datamodel_string: &str) -> Result<ast::SchemaAst, error::ErrorCollection> {
    ast::parser::parse(datamodel_string)
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

/// Renders an datamodel AST to a datamodel string. For internal use in tests only.
pub fn render_schema_ast_to_string(schema: &ast::SchemaAst) -> String {
    let mut writable_string = common::WritableString::new();
    render_schema_ast_to(&mut writable_string, schema, 2);
    writable_string.into()
}

/// Renders as a string into the stream.
fn render_schema_ast_to(stream: &mut dyn std::io::Write, schema: &ast::SchemaAst, ident_width: usize) {
    let mut renderer = ast::renderer::Renderer::new(stream, ident_width);
    renderer.render(schema);
}
