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

pub fn parsing_catch_all(token: &Token, kind: &str) {
    match token.as_rule() {
        Rule::comment | Rule::comment_and_new_line | Rule::comment_block => {}
        x => unreachable!(
            "Encountered impossible {} during parsing: {:?} {:?}",
            kind,
            &x,
            token.clone().tokens()
        ),
    }
}

pub type Token<'a> = pest::iterators::Pair<'a, Rule>;

pub trait TokenExtensions {
    fn first_child(&self) -> Token;

    fn filtered_children(&self) -> Vec<Token>;
}

// this is not implemented for Token because auto completion does not work then
impl TokenExtensions for pest::iterators::Pair<'_, Rule> {
    fn first_child(&self) -> Token<'_> {
        self.filtered_children().into_iter().next().unwrap()
    }

    fn filtered_children(&self) -> Vec<Token> {
        self.clone()
            .into_inner()
            .filter(|rule| {
                rule.as_rule() != Rule::BLOCK_CLOSE
                    && rule.as_rule() != Rule::BLOCK_OPEN
                    && rule.as_rule() != Rule::WHITESPACE
                    && rule.as_rule() != Rule::NEWLINE
            })
            .collect()
    }
}
