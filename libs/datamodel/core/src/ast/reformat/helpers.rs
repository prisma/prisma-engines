use crate::ast::parser::*;
use crate::ast::renderer::LineWriteable;

pub type Token<'a> = pest::iterators::Pair<'a, Rule>;

pub trait TokenExtensions {
    fn is_top_level_element(&self) -> bool;
}

impl TokenExtensions for Token<'_> {
    fn is_top_level_element(&self) -> bool {
        match self.as_rule() {
            Rule::model_declaration => true,
            Rule::enum_declaration => true,
            Rule::source_block => true,
            Rule::generator_block => true,
            Rule::type_alias => true,
            Rule::comment_block => true,
            _ => false,
        }
    }
}

pub fn comment(target: &mut dyn LineWriteable, comment_text: &str) {
    let trimmed = if comment_text.ends_with("\n") {
        &comment_text[0..comment_text.len() - 1] // slice away line break.
    } else {
        &comment_text
    };

    let trimmed = trimmed.trim();

    if !target.line_empty() {
        // Prefix with whitespace seperator.
        target.write(trimmed);
    } else {
        target.write(trimmed);
    }
    target.end_line();
}
