use super::{
    parse_composite_type::parse_composite_type, parse_enum::parse_enum, parse_model::parse_model,
    parse_source_and_generator::parse_config_block, parse_view::parse_view, PrismaDatamodelParser, Rule,
};
use crate::ast::*;
use diagnostics::{DatamodelError, Diagnostics};
use pest::Parser;

/// Parse a PSL string and return its AST.
pub fn parse_schema(datamodel_string: &str, diagnostics: &mut Diagnostics) -> SchemaAst {
    let datamodel_result = PrismaDatamodelParser::parse(Rule::schema, datamodel_string);

    match datamodel_result {
        Ok(mut datamodel_wrapped) => {
            let datamodel = datamodel_wrapped.next().unwrap();
            let mut top_level_definitions: Vec<Top> = vec![];
            let mut pending_block_comment = None;
            let mut pairs = datamodel.into_inner().peekable();

            while let Some(current) = pairs.next() {
                match current.as_rule() {
                    Rule::model_declaration => {
                        let keyword = current.clone().into_inner().find(|pair| matches!(pair.as_rule(), Rule::TYPE_KEYWORD | Rule::MODEL_KEYWORD | Rule::VIEW_KEYWORD) ).expect("Expected model, type or view keyword");

                        match keyword.as_rule() {
                            Rule::TYPE_KEYWORD => {
                                top_level_definitions.push(Top::CompositeType(parse_composite_type(current, pending_block_comment.take(), diagnostics)))
                            }
                            Rule::MODEL_KEYWORD => {
                                top_level_definitions.push(Top::Model(parse_model(current, pending_block_comment.take(), diagnostics)))
                            }
                            Rule::VIEW_KEYWORD => {
                                top_level_definitions.push(Top::Model(parse_view(current, pending_block_comment.take(), diagnostics)))
                            }
                            _ => unreachable!(),
                        }

                    },
                    Rule::enum_declaration => top_level_definitions.push(Top::Enum(parse_enum(current,pending_block_comment.take(),  diagnostics))),
                    Rule::config_block => {
                        top_level_definitions.push(parse_config_block(current, diagnostics));
                    },
                    Rule::type_alias => {
                        let error = DatamodelError::new_validation_error(
                            "Invalid type definition. Please check the documentation in https://pris.ly/d/composite-types",
                            current.as_span().into()
                        );

                        diagnostics.push_error(error);
                    }
                    Rule::comment_block => {
                        match pairs.peek().map(|b| b.as_rule()) {
                            Some(Rule::empty_lines) => {
                                // free floating
                            }
                            Some(Rule::model_declaration) | Some(Rule::enum_declaration) | Some(Rule::config_block) => {
                                pending_block_comment = Some(current);
                            }
                            _ => (),
                        }
                    },
                    Rule::EOI => {}
                    Rule::CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                        "This line is invalid. It does not start with any known Prisma schema keyword.",
                        current.as_span().into(),
                    )),
                    // TODO: Add view when we want it to be more visible as a feature.
                    Rule::arbitrary_block => diagnostics.push_error(DatamodelError::new_validation_error(
                        "This block is invalid. It does not start with any known Prisma schema keyword. Valid keywords include \'model\', \'enum\', \'type\', \'datasource\' and \'generator\'.",
                        current.as_span().into(),
                    )),
                    Rule::empty_lines => (),
                    _ => unreachable!(),
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
        write!(out, "{positive:?}").unwrap();
    }

    out
}
