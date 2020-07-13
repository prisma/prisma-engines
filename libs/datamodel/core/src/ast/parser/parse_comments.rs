use super::{
    helpers::{parsing_catch_all, Token, TokenExtensions},
    Rule,
};
use crate::ast::Comment;

pub fn parse_comment_block(token: &Token) -> Comment {
    let mut comments: Vec<String> = Vec::new();
    for comment in token.clone().into_inner() {
        match comment.as_rule() {
            Rule::doc_comment | Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&comment)),
            Rule::comment | Rule::comment_and_new_line => {}
            _ => parsing_catch_all(&comment, "comment block"),
        }
    }

    Comment {
        text: comments.join("\n"),
    }
}

pub fn parse_doc_comment(token: &Token) -> String {
    let child = token.first_relevant_child();
    match child.as_rule() {
        Rule::doc_content => String::from(child.as_str().trim()),
        Rule::doc_comment => parse_doc_comment(&child),
        x => unreachable!(
            "Encountered impossible doc comment during parsing: {:?}, {:?}",
            x,
            child.tokens()
        ),
    }
}

pub fn doc_comments_to_string(comments: &[String]) -> Option<Comment> {
    if comments.is_empty() {
        None
    } else {
        Some(Comment {
            text: comments.join("\n"),
        })
    }
}
