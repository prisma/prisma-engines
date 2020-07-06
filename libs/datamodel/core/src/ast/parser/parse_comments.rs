use super::{helpers::parsing_catch_all, Rule};
use crate::ast::Comment;

pub fn parse_comment_block(token: &pest::iterators::Pair<'_, Rule>) -> Comment {
    let mut comments: Vec<String> = Vec::new();
    for comment in token.clone().into_inner() {
        match comment.as_rule() {
            Rule::doc_comment | Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&comment)),
            Rule::comment | Rule::comment_and_new_line => {}
            _ => parsing_catch_all(&comment),
        }
    }

    Comment {
        text: comments.join("\n"),
    }
}

// Documentation parsing
pub fn parse_doc_comment(token: &pest::iterators::Pair<'_, Rule>) -> String {
    match_first! { token, current,
        Rule::doc_content => {
            String::from(current.as_str().trim())
        },
        Rule::doc_comment => {
            parse_doc_comment(&current)
        },
        x => unreachable!("Encountered impossible doc comment during parsing: {:?}, {:?}", x, current.tokens())
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
