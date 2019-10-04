use super::values::values_match;
use datamodel::ast;

pub(crate) fn directives_match_exactly(previous: &ast::Directive, next: &ast::Directive) -> bool {
    previous.name.name == next.name.name && argument_lists_match_exactly(&previous.arguments, &next.arguments)
}

/// Extract the unnamed argument of the specified directive as a string value, if possible.
pub(crate) fn get_directive_string_value<'a>(
    directive_name: &str,
    directives: &'a [ast::Directive],
) -> Option<&'a str> {
    directives
        .iter()
        .find(|directive| directive.name.name == directive_name)
        .and_then(|directive| directive.arguments.iter().next())
        .and_then(|argument| match &argument.value {
            ast::Value::StringValue(value, _span) => Some(value.as_str()),
            _ => None,
        })
}

fn argument_lists_match_exactly(previous: &[ast::Argument], next: &[ast::Argument]) -> bool {
    previous.len() == next.len()
        && previous.iter().all(|previous_argument| {
            next.iter()
                .find(|next_argument| arguments_match(previous_argument, next_argument))
                .is_some()
        })
}

fn arguments_match(previous: &ast::Argument, next: &ast::Argument) -> bool {
    previous.name.name == next.name.name && values_match(&previous.value, &next.value)
}
