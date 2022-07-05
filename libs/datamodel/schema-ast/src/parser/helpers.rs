use super::Rule;
use crate::ast::{Identifier, Span};

pub type Pair<'a> = pest::iterators::Pair<'a, Rule>;

#[track_caller]
pub fn parsing_catch_all(token: &Pair<'_>, kind: &str) {
    match token.as_rule() {
        Rule::empty_lines | Rule::trailing_comment | Rule::comment_block => {}
        x => unreachable!(
            "Encountered impossible {} during parsing: {:?} {:?}",
            kind,
            &x,
            token.clone().tokens()
        ),
    }
}

pub trait ToIdentifier {
    fn to_id(&self) -> Identifier;
}

// this is not implemented for Token because auto completion does not work then
impl ToIdentifier for pest::iterators::Pair<'_, Rule> {
    fn to_id(&self) -> Identifier {
        Identifier {
            name: String::from(self.as_str()),
            span: Span::from(self.as_span()),
        }
    }
}
