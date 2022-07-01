use super::{
    helpers::{parsing_catch_all, Pair, ToIdentifier},
    Rule,
};
use crate::{ast::*, parser::parse_arguments::parse_arguments_list};

pub(crate) fn parse_attribute(pair: Pair<'_>, diagnostics: &mut diagnostics::Diagnostics) -> Attribute {
    let span = Span::from(pair.as_span());
    let mut name = Identifier {
        name: String::new(),
        span,
    };
    let mut arguments: ArgumentsList = ArgumentsList::default();

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::attribute_name => name = current.to_id(),
            Rule::arguments_list => parse_arguments_list(current, &mut arguments, diagnostics),
            _ => parsing_catch_all(&current, "attribute"),
        }
    }

    Attribute { name, arguments, span }
}
