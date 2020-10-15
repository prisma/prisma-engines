use pest::Parser;

use super::{
    helpers::{parsing_catch_all, TokenExtensions},
    parse_enum::parse_enum,
    parse_model::parse_model,
    parse_source_and_generator::{parse_generator, parse_source},
    parse_types::parse_type_alias,
    PrismaDatamodelParser, Rule,
};
use crate::ast::*;
use crate::diagnostics::{DatamodelError, Diagnostics};

/// Parses a Prisma V2 datamodel document into an internal AST representation.
pub fn parse_schema(datamodel_string: &str) -> Result<SchemaAst, Diagnostics> {
    let mut errors = Diagnostics::new();
    let datamodel_result = PrismaDatamodelParser::parse(Rule::schema, datamodel_string);

    match datamodel_result {
        Ok(mut datamodel_wrapped) => {
            let datamodel = datamodel_wrapped.next().unwrap();
            let mut top_level_definitions: Vec<Top> = vec![];
            for current in datamodel.relevant_children() {
                match current.as_rule() {
                    Rule::model_declaration => match parse_model(&current) {
                        Ok(model) => top_level_definitions.push(Top::Model(model)),
                        Err(mut err) => errors.append(&mut err),
                    },
                    Rule::enum_declaration => match parse_enum(&current) {
                        Ok(enm) => top_level_definitions.push(Top::Enum(enm)),
                        Err(mut err) => errors.append(&mut err),
                    },
                    Rule::source_block => match parse_source(&current) {
                        Ok(source) => top_level_definitions.push(Top::Source(source)),
                        Err(mut err) => errors.append(&mut err),
                    },
                    Rule::generator_block => match parse_generator(&current) {
                        Ok(generator) => top_level_definitions.push(Top::Generator(generator)),
                        Err(mut err) => errors.append(&mut err),
                    },
                    Rule::type_alias => top_level_definitions.push(Top::Type(parse_type_alias(&current))),
                    Rule::comment_block => (),
                    Rule::EOI => {}
                    Rule::CATCH_ALL => errors.push_error(DatamodelError::new_validation_error(
                        &"This line is invalid. It does not start with any known Prisma schema keyword.".to_string(),
                        Span::from_pest(current.as_span()),
                    )),
                    Rule::arbitrary_block => errors.push_error(DatamodelError::new_validation_error(
                        &"This block is invalid. It does not start with any known Prisma schema keyword. Valid keywords include \'model\', \'enum\', \'datasource\' and \'generator\'.".to_string(),
                        Span::from_pest(current.as_span()),
                    )),
                    _ => parsing_catch_all(&current, "datamodel"),
                }
            }

            errors.to_result()?;

            Ok(SchemaAst {
                tops: top_level_definitions,
            })
        }
        Err(err) => {
            let location = match err.location {
                pest::error::InputLocation::Pos(pos) => Span::new(pos, pos),
                pest::error::InputLocation::Span((from, to)) => Span::new(from, to),
            };

            let expected = match err.variant {
                pest::error::ErrorVariant::ParsingError { positives, .. } => get_expected_from_error(&positives),
                _ => panic!("Could not construct parsing error. This should never happend."),
            };

            errors.push_error(DatamodelError::new_parser_error(&expected, location));
            Err(errors)
        }
    }
}

fn get_expected_from_error(positives: &[Rule]) -> Vec<&'static str> {
    positives
        .iter()
        .map(|r| rule_to_string(*r))
        .filter(|s| s != &"")
        .collect()
}

fn rule_to_string(rule: Rule) -> &'static str {
    match rule {
        Rule::model_declaration => "model declaration",
        Rule::enum_declaration => "enum declaration",
        Rule::source_block => "source definition",
        Rule::generator_block => "generator definition",
        Rule::arbitrary_block => "arbitrary block",
        Rule::enum_value_declaration => "enum field declaration",
        Rule::block_level_attribute => "block level attribute",
        Rule::EOI => "end of input",
        Rule::non_empty_identifier => "alphanumeric identifier",
        Rule::maybe_empty_identifier => "alphanumeric identifier",
        Rule::numeric_literal => "numeric literal",
        Rule::string_literal => "string literal",
        Rule::boolean_literal => "boolean literal",
        Rule::constant_literal => "literal",
        Rule::array_expression => "array",
        Rule::expression => "expression",
        Rule::argument_name => "argument name",
        Rule::function => "function expression",
        Rule::argument_value => "argument value",
        Rule::argument => "argument",
        Rule::attribute_arguments => "attribute arguments",
        Rule::attribute_name => "attribute name",
        Rule::attribute => "attribute",
        Rule::optional_type => "optional type",
        Rule::base_type => "type",
        Rule::list_type => "list type",
        Rule::field_type => "field type",
        Rule::field_declaration => "field declaration",
        Rule::type_alias => "type alias",
        Rule::key_value => "configuration property",
        Rule::string_any => "any character",
        Rule::string_escaped_interpolation => "string interpolation",
        Rule::doc_comment => "documentation comment",
        Rule::doc_comment_and_new_line => "multi line documentation comment",
        Rule::comment => "comment",
        Rule::comment_and_new_line => "comment and new line",
        Rule::comment_block => "comment block",
        Rule::number => "number",

        // Those are helpers, so we get better error messages:
        Rule::BLOCK_OPEN => "Start of block (\"{\")",
        Rule::BLOCK_CLOSE => "End of block (\"}\")",
        Rule::MODEL_KEYWORD => "\"model\" keyword",
        Rule::TYPE_KEYWORD => "\"type\" keyword",
        Rule::ENUM_KEYWORD => "\"enum\" keyword",
        Rule::GENERATOR_KEYWORD => "\"generator\" keyword",
        Rule::DATASOURCE_KEYWORD => "\"datasource\" keyword",
        Rule::INTERPOLATION_START => "string interpolation start",
        Rule::INTERPOLATION_END => "string interpolation end",
        Rule::CATCH_ALL => "CATCH ALL",
        Rule::BLOCK_LEVEL_CATCH_ALL => "BLOCK LEVEL CATCH ALL",

        // Those are top level things and will never surface.
        Rule::schema => "schema",
        Rule::string_interpolated => "string interpolated",

        // Legacy stuff should never be suggested
        Rule::LEGACY_COLON => "",
        Rule::legacy_list_type => "",
        Rule::legacy_required_type => "",
        Rule::unsupported_optional_list_type => "",

        // Atomic and helper rules should not surface, we still add them for debugging.
        Rule::WHITESPACE => "",
        Rule::NEWLINE => "newline",
        Rule::string_escaped_predefined => "escaped unicode char",
        Rule::string_escape => "escaped unicode char",
        Rule::string_interpolate_escape => "string interpolation",
        Rule::string_raw => "unescaped string",
        Rule::string_content => "string contents",
        Rule::boolean_true => "boolean true",
        Rule::boolean_false => "boolean false",
        Rule::doc_content => "documentation comment content",
    }
}
