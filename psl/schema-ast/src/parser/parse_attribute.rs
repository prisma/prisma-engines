use super::{
    Rule,
    helpers::{Pair, parsing_catch_all},
};
use crate::{ast::*, parser::parse_arguments::parse_arguments_list};
use diagnostics::FileId;

pub(crate) fn parse_attribute(
    pair: Pair<'_>,
    diagnostics: &mut diagnostics::Diagnostics,
    file_id: FileId,
) -> Attribute {
    let span = Span::from((file_id, pair.as_span()));
    let mut name = None;
    let mut arguments: ArgumentsList = ArgumentsList::default();

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::path => name = Some(Identifier::new(current, file_id)),
            Rule::arguments_list => parse_arguments_list(current, &mut arguments, diagnostics, file_id),
            _ => parsing_catch_all(&current, "attribute"),
        }
    }

    let name = name.unwrap();
    Attribute { name, arguments, span }
}
