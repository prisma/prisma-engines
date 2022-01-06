use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    Rule,
};
use crate::{ast::*, parser::parse_arguments::parse_arguments_list};

pub fn parse_attribute(token: &Token<'_>) -> Attribute {
    let mut name: Option<Identifier> = None;
    let mut arguments: ArgumentsList = ArgumentsList::default();

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::attribute => return parse_attribute(&current),
            Rule::attribute_name => name = Some(current.to_id()),
            Rule::arguments_list => parse_arguments_list(&current, &mut arguments),
            _ => parsing_catch_all(&current, "attribute"),
        }
    }

    match name {
        Some(name) => Attribute {
            name,
            arguments,
            span: Span::from(token.as_span()),
        },
        _ => panic!("Encountered impossible type during parsing: {:?}", token.as_str()),
    }
}
