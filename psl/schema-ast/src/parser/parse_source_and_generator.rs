use super::{
    helpers::{parsing_catch_all, Pair},
    parse_comments::*,
    parse_expression::parse_expression,
    Rule,
};
use crate::ast::*;
use diagnostics::{DatamodelError, Diagnostics};

#[track_caller]
pub(crate) fn parse_config_block(pair: Pair<'_>, diagnostics: &mut Diagnostics) -> Top {
    let pair_span = pair.as_span();
    let mut name: Option<Identifier> = None;
    let mut properties = Vec::new();
    let mut comment: Option<Comment> = None;
    let mut kw = None;
    let mut inner_span: Option<Span> = None;

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::config_contents => {
                inner_span = Some(current.as_span().into());
                for item in current.into_inner() {
                    match item.as_rule() {
                        Rule::key_value => properties.push(parse_key_value(item, diagnostics)),
                        Rule::comment_block => comment = parse_comment_block(item),
                        Rule::BLOCK_LEVEL_CATCH_ALL => {
                            let msg = format!(
                                "This line is not a valid definition within a {}.",
                                kw.unwrap_or("configuration block")
                            );

                            let err = DatamodelError::new_validation_error(&msg, item.as_span().into());
                            diagnostics.push_error(err);
                        }
                        _ => parsing_catch_all(&item, "source"),
                    }
                }
            }
            Rule::identifier => name = Some(current.into()),
            Rule::DATASOURCE_KEYWORD | Rule::GENERATOR_KEYWORD => kw = Some(current.as_str()),
            Rule::BLOCK_OPEN | Rule::BLOCK_CLOSE => {}

            _ => parsing_catch_all(&current, "source"),
        }
    }

    match kw {
        Some("datasource") => Top::Source(SourceConfig {
            name: name.unwrap(),
            properties,
            documentation: comment,
            span: Span::from(pair_span),
            inner_span: inner_span.unwrap(),
        }),
        Some("generator") => Top::Generator(GeneratorConfig {
            name: name.unwrap(),
            properties,
            documentation: comment,
            span: Span::from(pair_span),
        }),
        _ => unreachable!(),
    }
}

fn parse_key_value(pair: Pair<'_>, diagnostics: &mut Diagnostics) -> ConfigBlockProperty {
    let mut name: Option<Identifier> = None;
    let mut value: Option<Expression> = None;
    let (pair_span, pair_str) = (pair.as_span(), pair.as_str());

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::identifier => name = Some(current.into()),
            Rule::expression => value = Some(parse_expression(current, diagnostics)),
            Rule::trailing_comment => (),
            _ => unreachable!(
                "Encountered impossible source property declaration during parsing: {:?}",
                current.tokens()
            ),
        }
    }

    match (name, value) {
        (Some(name), value) => ConfigBlockProperty {
            name,
            value,
            span: Span::from(pair_span),
        },
        _ => unreachable!(
            "Encountered impossible source property declaration during parsing: {:?}",
            pair_str,
        ),
    }
}
