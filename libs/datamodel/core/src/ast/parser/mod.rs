use pest::Parser;

// This is how PEG grammars work:
// https://pest.rs/book/grammars/peg.html

// This is the basic syntax of Pest grammar files:
// https://pest.rs/book/grammars/syntax.html#cheat-sheet

#[derive(Parser)]
#[grammar = "ast/parser/datamodel.pest"]
pub struct PrismaDatamodelParser;

use crate::ast::*;
use crate::error::{DatamodelError, ErrorCollection};

trait ToIdentifier {
    fn to_id(&self) -> Identifier;
}

impl ToIdentifier for pest::iterators::Pair<'_, Rule> {
    fn to_id(&self) -> Identifier {
        Identifier {
            name: String::from(self.as_str()),
            span: Span::from_pest(self.as_span()),
        }
    }
}

fn parse_string_literal(token: &pest::iterators::Pair<'_, Rule>) -> String {
    return match_first! { token, current,
        Rule::string_content => current.as_str().to_string(),
        _ => unreachable!("Encountered impossible string content during parsing: {:?}", current.tokens())
    };
}

// Expressions

/// Parses an expression, given a Pest parser token.
pub fn parse_expression(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    return match_first! { token, current,
        Rule::numeric_literal => Expression::NumericValue(current.as_str().to_string(), Span::from_pest(current.as_span())),
        Rule::string_literal => Expression::StringValue(parse_string_literal(&current), Span::from_pest(current.as_span())),
        Rule::boolean_literal => Expression::BooleanValue(current.as_str().to_string(), Span::from_pest(current.as_span())),
        Rule::constant_literal => Expression::ConstantValue(current.as_str().to_string(), Span::from_pest(current.as_span())),
        Rule::function => parse_function(&current),
        Rule::array_expression => parse_array(&current),
        _ => unreachable!("Encountered impossible literal during parsing: {:?}", current.tokens())
    };
}

fn parse_function(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    let mut name: Option<String> = None;
    let mut arguments: Vec<Expression> = vec![];

    match_children! { token, current,
        Rule::non_empty_identifier => name = Some(current.as_str().to_string()),
        Rule::argument_value => arguments.push(parse_arg_value(&current)),
        _ => unreachable!("Encountered impossible function during parsing: {:?}", current.tokens())
    };

    match name {
        Some(name) => Expression::Function(name, arguments, Span::from_pest(token.as_span())),
        _ => unreachable!("Encountered impossible function during parsing: {:?}", token.as_str()),
    }
}

fn parse_array(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    let mut elements: Vec<Expression> = vec![];

    match_children! { token, current,
        Rule::expression => elements.push(parse_expression(&current)),
        _ => unreachable!("Encountered impossible array during parsing: {:?}", current.tokens())
    };

    Expression::Array(elements, Span::from_pest(token.as_span()))
}

fn parse_arg_value(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    match_first! { token, current,
        Rule::expression => parse_expression(&current),
        _ => unreachable!("Encountered impossible value during parsing: {:?}", current.tokens())
    }
}

// Documentation parsing

fn parse_doc_comment(token: &pest::iterators::Pair<'_, Rule>) -> String {
    match_first! { token, current,
        Rule::doc_content => {
            String::from(current.as_str().trim())
        },
        Rule::doc_comment => {
            parse_doc_comment(&current)
        },
        x => unreachable!("Encountered impossible doc comment during parsing: {:?}, {:?}", x, current.tokens())
    }
}

fn doc_comments_to_string(comments: &[String]) -> Option<Comment> {
    if comments.is_empty() {
        None
    } else {
        Some(Comment {
            text: comments.join("\n"),
        })
    }
}

// Directive parsing

fn parse_directive(token: &pest::iterators::Pair<'_, Rule>) -> Directive {
    let mut name: Option<Identifier> = None;
    let mut arguments: Vec<Argument> = vec![];

    match_children! { token, current,
        Rule::directive => return parse_directive(&current),
        Rule::directive_name => name = Some(current.to_id()),
        Rule::directive_arguments => parse_directive_args(&current, &mut arguments),
        _ => unreachable!("Encountered impossible directive during parsing: {:?} \n {:?}", token, current.tokens())
    };

    match name {
        Some(name) => Directive {
            name,
            arguments,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!("Encountered impossible type during parsing: {:?}", token.as_str()),
    }
}

fn parse_directive_args(token: &pest::iterators::Pair<'_, Rule>, arguments: &mut Vec<Argument>) {
    match_children! { token, current,
        // This is a named arg.
        Rule::argument => arguments.push(parse_directive_arg(&current)),
        // This is a an unnamed arg.
        Rule::argument_value => arguments.push(Argument {
            name: Identifier::new(""),
            value: parse_arg_value(&current),
            span: Span::from_pest(current.as_span())
        }),
        _ => unreachable!("Encountered impossible directive argument during parsing: {:?}", current.tokens())
    }
}

fn parse_directive_arg(token: &pest::iterators::Pair<'_, Rule>) -> Argument {
    let mut name: Option<Identifier> = None;
    let mut argument: Option<Expression> = None;

    match_children! { token, current,
        Rule::argument_name => name = Some(current.to_id()),
        Rule::argument_value => argument = Some(parse_arg_value(&current)),
        _ => unreachable!("Encountered impossible directive argument during parsing: {:?}", current.tokens())
    };

    match (name, argument) {
        (Some(name), Some(value)) => Argument {
            name,
            value,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible directive arg during parsing: {:?}",
            token.as_str()
        ),
    }
}

// Base type parsing
fn parse_base_type(token: &pest::iterators::Pair<'_, Rule>) -> String {
    match_first! { token, current,
        Rule::non_empty_identifier => current.as_str().to_string(),
        _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens())
    }
}

fn parse_field_type(token: &pest::iterators::Pair<'_, Rule>) -> Result<(FieldArity, String), DatamodelError> {
    match_first! { token, current,
        Rule::optional_type => Ok((FieldArity::Optional, parse_base_type(&current))),
        Rule::base_type =>  Ok((FieldArity::Required, parse_base_type(&current))),
        Rule::list_type =>  Ok((FieldArity::List, parse_base_type(&current))),
        Rule::legacy_required_type => Err(DatamodelError::new_legacy_parser_error(
            "Fields are required by default, `!` is no longer required.",
            Span::from_pest(current.as_span())
        )),
        Rule::legacy_list_type => Err(DatamodelError::new_legacy_parser_error(
            "To specify a list, please use `Type[]` instead of `[Type]`.",
            Span::from_pest(current.as_span())
        )),
        Rule::unsupported_optional_list_type => Err(DatamodelError::new_legacy_parser_error(
            "Optional lists are not supported. Use either `Type[]` or `Type?`.",
            Span::from_pest(current.as_span())
        )),
        _ => unreachable!("Encountered impossible field during parsing: {:?}", current.tokens())
    }
}

fn parse_field(model_name: &str, token: &pest::iterators::Pair<'_, Rule>) -> Result<Field, DatamodelError> {
    let mut name: Option<Identifier> = None;
    let mut directives: Vec<Directive> = Vec::new();
    let mut field_type: Option<((FieldArity, String), Span)> = None;
    let mut comments: Vec<String> = Vec::new();

    match_children! { token, current,
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::field_type => field_type = Some(
            (
                parse_field_type(&current)?,
                Span::from_pest(current.as_span())
            )
        ),
        Rule::LEGACY_COLON => return Err(DatamodelError::new_legacy_parser_error(
            "Field declarations don't require a `:`.",
            Span::from_pest(current.as_span()))),
        Rule::directive => directives.push(parse_directive(&current)),
        Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
        Rule::doc_comment => comments.push(parse_doc_comment(&current)),
        _ => unreachable!("Encountered impossible field declaration during parsing: {:?}", current.tokens())
    }

    match (name, field_type) {
        (Some(name), Some(((arity, field_type), field_type_span))) => Ok(Field {
            field_type: Identifier {
                name: field_type,
                span: field_type_span,
            },
            name,
            arity,
            default_value: None,
            directives,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
            is_commented_out: false,
        }),
        _ => Err(DatamodelError::new_model_validation_error(
            &format!("This field declaration is invalid. It is either missing a name or a type."),
            model_name,
            Span::from_pest(token.as_span()),
        )),
    }
}
// Model parsing
fn parse_model(token: &pest::iterators::Pair<'_, Rule>) -> Result<Model, ErrorCollection> {
    let mut errors = ErrorCollection::new();
    let mut name: Option<Identifier> = None;
    let mut directives: Vec<Directive> = vec![];
    let mut fields: Vec<Field> = vec![];
    let mut comments: Vec<String> = Vec::new();

    match_children! { token, current,
        Rule::MODEL_KEYWORD => { },
        Rule::TYPE_KEYWORD => { errors.push(
            DatamodelError::new_legacy_parser_error(
                "Model declarations have to be indicated with the `model` keyword.",
                Span::from_pest(current.as_span()))
        ) },
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::directive => directives.push(parse_directive(&current)),
        Rule::field_declaration => {
            match parse_field(&name.as_ref().unwrap().name, &current) {
                Ok(field) => fields.push(field),
                Err(err) => errors.push(err)
            }
        },
        Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
        Rule::doc_comment => comments.push(parse_doc_comment(&current)),
        Rule::UNTIL_END_OF_LINE => {},
        _ => unreachable!("Encountered impossible model declaration during parsing: {:?}", current.tokens())
    }

    errors.ok()?;

    match name {
        Some(name) => Ok(Model {
            name,
            fields,
            directives,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
            commented_out: false,
        }),
        _ => panic!(
            "Encountered impossible model declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}

// Enum parsing
fn parse_enum(token: &pest::iterators::Pair<'_, Rule>) -> Result<Enum, ErrorCollection> {
    let mut errors = ErrorCollection::new();
    let mut name: Option<Identifier> = None;
    let mut directives: Vec<Directive> = vec![];
    let mut values: Vec<EnumValue> = vec![];
    let mut comments: Vec<String> = Vec::new();

    match_children! { token, current,
        Rule::ENUM_KEYWORD => { },
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::block_level_directive => directives.push(parse_directive(&current)),
        Rule::enum_field_declaration => {
            match parse_enum_value(&name.as_ref().unwrap().name, &current) {
                Ok(enum_value) => values.push(enum_value),
                Err(err) => errors.push(err)
            }
        },
        Rule::doc_comment => comments.push(parse_doc_comment(&current)),
        Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
        _ => unreachable!("Encountered impossible enum declaration during parsing: {:?}", current.tokens())
    }

    errors.ok()?;

    match name {
        Some(name) => Ok(Enum {
            name,
            values,
            directives,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
        }),
        _ => panic!(
            "Encountered impossible enum declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}

// Enum value parsing
fn parse_enum_value(enum_name: &str, token: &pest::iterators::Pair<'_, Rule>) -> Result<EnumValue, DatamodelError> {
    let mut name: Option<Identifier> = None;
    let mut directives: Vec<Directive> = vec![];
    let mut comments: Vec<String> = vec![];

    //todo validate that the identifier is valid???
    match_children! { token, current,
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::maybe_empty_identifier => name = Some(current.to_id()),
        Rule::directive => directives.push(parse_directive(&current)),
        Rule::number => {
            return Err(DatamodelError::new_enum_validation_error(
                &format!("The enum value `{}` is not valid. Enum values must not start with a number.", current.as_str()),
                enum_name,
                Span::from_pest(token.as_span()))
            );
        },
        Rule::doc_comment => {
            comments.push(parse_doc_comment(&current));
        },
        Rule::doc_comment_and_new_line => {
            comments.push(parse_doc_comment(&current));
        },
        _ => unreachable!("Encountered impossible enum value declaration during parsing: {:?}", current.as_str())
    }

    match name {
        Some(name) => Ok(EnumValue {
            name,
            directives,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
            commented_out: false,
        }),
        _ => panic!(
            "Encountered impossible enum value declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}

fn parse_key_value(token: &pest::iterators::Pair<'_, Rule>) -> Argument {
    let mut name: Option<Identifier> = None;
    let mut value: Option<Expression> = None;

    match_children! { token, current,
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::expression => value = Some(parse_expression(&current)),
        _ => unreachable!("Encountered impossible source property declaration during parsing: {:?}", current.tokens())
    }

    match (name, value) {
        (Some(name), Some(value)) => Argument {
            name,
            value,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible source property declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}

// Source parsing
fn parse_source(token: &pest::iterators::Pair<'_, Rule>) -> SourceConfig {
    let mut name: Option<Identifier> = None;
    let mut properties: Vec<Argument> = vec![];
    let mut comments: Vec<String> = Vec::new();

    match_children! { token, current,
        Rule::DATASOURCE_KEYWORD => { },
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::key_value => properties.push(parse_key_value(&current)),
        Rule::doc_comment => comments.push(parse_doc_comment(&current)),
        Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
        _ => unreachable!("Encountered impossible source declaration during parsing: {:?}", current.tokens())
    };

    match name {
        Some(name) => SourceConfig {
            name,
            properties,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible source declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}

// Generator parsing
fn parse_generator(token: &pest::iterators::Pair<'_, Rule>) -> GeneratorConfig {
    let mut name: Option<Identifier> = None;
    let mut properties: Vec<Argument> = vec![];
    let mut comments: Vec<String> = Vec::new();

    match_children! { token, current,
        Rule::GENERATOR_KEYWORD => { },
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::key_value => properties.push(parse_key_value(&current)),
        Rule::doc_comment => comments.push(parse_doc_comment(&current)),
        Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
        _ => unreachable!("Encountered impossible generator declaration during parsing: {:?}", current.tokens())
    };

    match name {
        Some(name) => GeneratorConfig {
            name,
            properties,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible generator declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}

// Custom type parsing
fn parse_type(token: &pest::iterators::Pair<'_, Rule>) -> Field {
    let mut name: Option<Identifier> = None;
    let mut directives: Vec<Directive> = vec![];
    let mut base_type: Option<(String, Span)> = None;
    let mut comments: Vec<String> = Vec::new();

    match_children! { token, current,
        Rule::TYPE_KEYWORD => { },
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::base_type => {
            base_type = Some((parse_base_type(&current), Span::from_pest(current.as_span())))
        },
        Rule::directive => directives.push(parse_directive(&current)),
        Rule::doc_comment => comments.push(parse_doc_comment(&current)),
        Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
        _ => unreachable!("Encountered impossible custom type during parsing: {:?}", current.tokens())
    }

    match (name, base_type) {
        (Some(name), Some((field_type, field_type_span))) => Field {
            field_type: Identifier {
                name: field_type,
                span: field_type_span,
            },
            name,
            arity: FieldArity::Required,
            default_value: None,
            directives,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
            is_commented_out: false,
        },
        _ => panic!(
            "Encountered impossible custom type declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}

// Whole datamodel parsing

/// Parses a Prisma V2 datamodel document into an internal AST representation.
pub fn parse(datamodel_string: &str) -> Result<SchemaAst, ErrorCollection> {
    let mut errors = ErrorCollection::new();
    let datamodel_result = PrismaDatamodelParser::parse(Rule::datamodel, datamodel_string);

    match datamodel_result {
        Ok(mut datamodel_wrapped) => {
            let datamodel = datamodel_wrapped.next().unwrap();
            let mut top_level_definitions: Vec<Top> = vec![];

            match_children! { datamodel, current,
                Rule::model_declaration => match parse_model(&current) {
                    Ok(model) => top_level_definitions.push(Top::Model(model)),
                    Err(mut err) => errors.append(&mut err)
                },
                Rule::enum_declaration => match parse_enum(&current){
                    Ok(enm) => top_level_definitions.push(Top::Enum(enm)),
                    Err(mut err) => errors.append(&mut err)
                },
                Rule::source_block => top_level_definitions.push(Top::Source(parse_source(&current))),
                Rule::generator_block => top_level_definitions.push(Top::Generator(parse_generator(&current))),
                Rule::type_declaration => top_level_definitions.push(Top::Type(parse_type(&current))),
                Rule::doc_comment => (),
                Rule::EOI => {},
                Rule::CATCH_ALL => {
                    errors.push(DatamodelError::new_validation_error(
                        &format!("This line is invalid. It does not start with any known Prisma schema keyword."),
                        Span::from_pest(current.as_span()))
                    )
                },
                Rule::arbitrary_block => {
                    errors.push(DatamodelError::new_validation_error(
                        &format!("This block is invalid. It does not start with any known Prisma schema keyword."),
                        Span::from_pest(current.as_span()))
                    )
                },
                _ => panic!("Encountered impossible datamodel declaration during parsing: {:?}", current.tokens())
            }

            errors.ok()?;

            Ok(SchemaAst {
                tops: top_level_definitions,
            })
        }
        Err(err) => {
            dbg!(&err);
            let location = match err.location {
                pest::error::InputLocation::Pos(pos) => Span::new(pos, pos),
                pest::error::InputLocation::Span((from, to)) => Span::new(from, to),
            };

            let expected = match err.variant {
                pest::error::ErrorVariant::ParsingError { positives, .. } => get_expected_from_error(&positives),
                _ => panic!("Could not construct parsing error. This should never happend."),
            };

            errors.push(DatamodelError::new_parser_error(&expected, location));
            Err(errors)
        }
    }
}

pub fn get_expected_from_error(positives: &[Rule]) -> Vec<&'static str> {
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
        Rule::enum_field_declaration => "enum field declaration",
        Rule::block_level_directive => "block level directive",
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
        Rule::directive_arguments => "attribute arguments",
        Rule::directive_name => "directive name",
        Rule::directive => "directive",
        Rule::optional_type => "optional type",
        Rule::base_type => "type",
        Rule::list_type => "list type",
        Rule::field_type => "field type",
        Rule::field_declaration => "field declaration",
        Rule::type_declaration => "type declaration",
        Rule::key_value => "configuration property",
        Rule::string_any => "any character",
        Rule::string_escaped_interpolation => "string interpolation",
        Rule::doc_comment => "documentation comment",
        Rule::doc_comment_and_new_line => "multi line documentation comment",
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
        Rule::UNTIL_END_OF_LINE => "until end of line",
        Rule::CATCH_ALL => "CATCH ALL",

        // Those are top level things and will never surface.
        Rule::datamodel => "datamodel declaration",
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
