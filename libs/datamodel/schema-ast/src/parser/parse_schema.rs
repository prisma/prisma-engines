use super::{
    helpers::{parsing_catch_all, TokenExtensions},
    parse_composite_type::parse_composite_type,
    parse_enum::parse_enum,
    parse_model::parse_model,
    parse_source_and_generator::parse_config_block,
    parse_types::parse_type_alias,
    PrismaDatamodelParser, Rule,
};
use crate::ast::*;
use diagnostics::{DatamodelError, DatamodelWarning, Diagnostics};
use pest::Parser;

/// Parses a Prisma V2 datamodel document into an internal AST representation.
pub fn parse_schema(datamodel_string: &str, diagnostics: &mut Diagnostics) -> SchemaAst {
    let datamodel_result = PrismaDatamodelParser::parse(Rule::schema, datamodel_string);

    match datamodel_result {
        Ok(mut datamodel_wrapped) => {
            let datamodel = datamodel_wrapped.next().unwrap();
            let mut top_level_definitions: Vec<Top> = vec![];
            for current in datamodel.relevant_children() {
                match current.as_rule() {
                    Rule::model_declaration => {
                        let keyword = current.clone().into_inner().find(|pair| matches!(pair.as_rule(), Rule::TYPE_KEYWORD | Rule::MODEL_KEYWORD) ).expect("Expected model or type keyword");

                        match keyword.as_rule() {
                            Rule::TYPE_KEYWORD => {
                                top_level_definitions.push(Top::CompositeType(parse_composite_type(&current, diagnostics)))
                            }
                            Rule::MODEL_KEYWORD => {
                                top_level_definitions.push(Top::Model(parse_model(&current, diagnostics)))
                            }
                            _ => unreachable!(),
                        }

                    },
                    Rule::enum_declaration => top_level_definitions.push(Top::Enum(parse_enum(&current, diagnostics))),
                    Rule::config_block => {
                        top_level_definitions.push(parse_config_block(&current, diagnostics));
                    },
                    Rule::type_alias => {
                        diagnostics.push_warning(DatamodelWarning::DeprecatedTypeAlias { span: current.as_span().into() });
                        top_level_definitions.push(Top::Type(parse_type_alias(&current)))
                    }
                    Rule::comment_block => (),
                    Rule::EOI => {}
                    Rule::CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                        "This line is invalid. It does not start with any known Prisma schema keyword.".to_owned(),
                        current.as_span().into(),
                    )),
                    Rule::arbitrary_block => diagnostics.push_error(DatamodelError::new_validation_error(
                        "This block is invalid. It does not start with any known Prisma schema keyword. Valid keywords include \'model\', \'enum\', \'datasource\' and \'generator\'.".to_owned(),
                        current.as_span().into(),
                    )),
                    _ => parsing_catch_all(&current, "datamodel"),
                }
            }

            SchemaAst {
                tops: top_level_definitions,
            }
        }
        Err(err) => {
            let location: pest::Span<'_> = match err.location {
                pest::error::InputLocation::Pos(pos) => pest::Span::new(datamodel_string, pos, pos).unwrap(),
                pest::error::InputLocation::Span((from, to)) => pest::Span::new(datamodel_string, from, to).unwrap(),
            };

            let expected = match err.variant {
                pest::error::ErrorVariant::ParsingError { positives, .. } => get_expected_from_error(&positives),
                _ => panic!("Could not construct parsing error. This should never happend."),
            };

            diagnostics.push_error(DatamodelError::new_parser_error(expected, location.into()));

            SchemaAst { tops: Vec::new() }
        }
    }
}

fn get_expected_from_error(positives: &[Rule]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(positives.len() * 6);

    for positive in positives {
        write!(out, "{:?}", positive).unwrap();
    }

    out
}
