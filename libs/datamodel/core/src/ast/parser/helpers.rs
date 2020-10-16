use super::Rule;
use crate::ast::{Identifier, Span};

pub type Token<'a> = pest::iterators::Pair<'a, Rule>;

pub fn parsing_catch_all(token: &Token, kind: &str) {
    match token.as_rule() {
        Rule::comment | Rule::comment_and_new_line | Rule::comment_block | Rule::doc_comment_and_new_line => {}
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
            span: Span::from_pest(self.as_span()),
        }
    }
}

pub trait TokenExtensions {
    /// Gets the first child token that is relevant.
    /// Irrelevant Tokens are e.g. new lines which we do not want to match during parsing.
    fn first_relevant_child(&self) -> Token;

    /// Returns all child token of this Token that are relevant.
    /// Irrelevant Tokens are e.g. new lines which we do not want to match during parsing.
    fn relevant_children(&self) -> Vec<Token>;
}

// this is not implemented for Token because auto completion does not work then
impl TokenExtensions for pest::iterators::Pair<'_, Rule> {
    fn first_relevant_child(&self) -> Token<'_> {
        self.relevant_children()
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("Token `{}` had no children.", &self))
    }

    fn relevant_children(&self) -> Vec<Token> {
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
