use schema_ast::{parser::Rule, renderer::LineWriteable};

pub(super) type Token<'a> = pest::iterators::Pair<'a, Rule>;

pub(super) trait TokenExtensions {
    fn is_top_level_element(&self) -> bool;
}

impl TokenExtensions for Token<'_> {
    fn is_top_level_element(&self) -> bool {
        matches!(
            self.as_rule(),
            Rule::model_declaration
                | Rule::enum_declaration
                | Rule::config_block
                | Rule::type_alias
                | Rule::comment_block
        )
    }
}

pub(super) fn comment(target: &mut dyn LineWriteable, comment_text: &str) {
    let trimmed = strip_new_line(comment_text);
    let trimmed = trimmed.trim();

    target.write(trimmed);
    target.end_line();
}

pub(super) fn strip_new_line(str: &str) -> &str {
    if str.ends_with('\n') {
        &str[0..str.len() - 1] // slice away line break.
    } else {
        str
    }
}
