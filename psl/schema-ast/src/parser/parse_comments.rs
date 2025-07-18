use super::{
    Rule,
    helpers::{Pair, parsing_catch_all},
};
use crate::ast::Comment;

pub(crate) fn parse_comment_block(token: Pair<'_>) -> Option<Comment> {
    debug_assert!(token.as_rule() == Rule::comment_block);
    let mut lines = Vec::new();
    for comment in token.clone().into_inner() {
        match comment.as_rule() {
            Rule::doc_comment => lines.push(parse_doc_comment(comment)),
            Rule::multi_line_comment => lines.push(parse_multi_line_comment(comment)),
            Rule::comment | Rule::NEWLINE | Rule::WHITESPACE => {}
            _ => parsing_catch_all(&comment, "comment block"),
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(Comment { text: lines.join("\n") })
    }
}

pub(crate) fn parse_trailing_comment(pair: Pair<'_>) -> Option<Comment> {
    debug_assert_eq!(pair.as_rule(), Rule::trailing_comment);
    let mut lines = Vec::new();
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::doc_comment => lines.push(parse_doc_comment(current)),
            Rule::multi_line_comment => lines.push(parse_multi_line_comment(current)),
            Rule::comment | Rule::NEWLINE | Rule::WHITESPACE => {}
            _ => parsing_catch_all(&current, "trailing comment"),
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(Comment { text: lines.join("\n") })
    }
}

pub(crate) fn parse_doc_comment(token: Pair<'_>) -> &str {
    let child = token.into_inner().next().unwrap();
    match child.as_rule() {
        Rule::doc_content => child.as_str().trim_start(),
        Rule::doc_comment => parse_doc_comment(child),
        x => unreachable!(
            "Encountered impossible doc comment during parsing: {:?}, {:?}",
            x,
            child.tokens()
        ),
    }
}

fn parse_multi_line_comment(token: Pair<'_>) -> &str {
    // The matched string includes `/*` and `*/`. Strip them off.
    let text = token.as_str();
    // Safety check: multi_line_comment matches `/* ... */`, so length is at least 4.
    text[2..text.len() - 2].trim()
}
