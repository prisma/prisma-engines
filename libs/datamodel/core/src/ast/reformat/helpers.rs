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
    let trimmed = strip_new_line(&comment_text);
    let trimmed = trimmed.trim();

    target.write(trimmed);
    target.end_line();
}

pub fn strip_new_line(str: &str) -> &str {
    if str.ends_with("\n") {
        &str[0..str.len() - 1] // slice away line break.
    } else {
        &str
    }
}
