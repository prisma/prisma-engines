use super::Rule;
use crate::ast::{Identifier, Span};

pub trait ToIdentifier {
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

pub fn parsing_catch_all(token: &pest::iterators::Pair<'_, Rule>) {
    match token.as_rule() {
        Rule::comment | Rule::comment_and_new_line | Rule::comment_block => {}
        x => unreachable!(
            "Encountered impossible field declaration during parsing: {:?} {:?}",
            &x,
            token.clone().tokens()
        ),
    }
}
