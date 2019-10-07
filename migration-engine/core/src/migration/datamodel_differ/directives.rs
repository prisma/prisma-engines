use super::values::values_match;
use datamodel::ast;

pub(crate) struct DirectiveDiffer<'a> {
    pub(crate) previous: &'a ast::Directive,
    pub(crate) next: &'a ast::Directive,
}

impl<'a> DirectiveDiffer<'a> {
    fn previous_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.previous.arguments.iter()
    }

    fn next_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.next.arguments.iter()
    }

    pub(crate) fn deleted_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.previous_arguments().filter(move |previous_argument| {
            self.next_arguments()
                .find(|next_argument| arguments_match(previous_argument, next_argument))
                .is_none()
        })
    }

    pub(crate) fn created_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.next_arguments().filter(move |next_argument| {
            self.previous_arguments()
                .find(|previous_argument| arguments_match(previous_argument, next_argument))
                .is_none()
        })
    }

    pub(crate) fn argument_pairs(&self) -> impl Iterator<Item = (&ast::Argument, &ast::Argument)> {
        self.previous_arguments().filter_map(move |previous_argument| {
            self.next_arguments()
                .find(|next_argument| arguments_match(previous_argument, next_argument))
                .map(|next_argument| (previous_argument, next_argument))
        })
    }
}

pub(crate) fn directives_match_exactly(previous: &ast::Directive, next: &ast::Directive) -> bool {
    previous.name.name == next.name.name && argument_lists_match_exactly(&previous.arguments, &next.arguments)
}

fn argument_lists_match_exactly(previous: &[ast::Argument], next: &[ast::Argument]) -> bool {
    previous.len() == next.len()
        && previous.iter().all(|previous_argument| {
            next.iter()
                .find(|next_argument| arguments_match_exactly(previous_argument, next_argument))
                .is_some()
        })
}

fn arguments_match_exactly(previous: &ast::Argument, next: &ast::Argument) -> bool {
    previous.name.name == next.name.name && values_match(&previous.value, &next.value)
}

fn arguments_match(previous: &ast::Argument, next: &ast::Argument) -> bool {
    previous.name.name == next.name.name
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

#[cfg(test)]
mod tests {
    use super::super::{ModelDiffer, TopDiffer};
    use super::*;
    use datamodel::parse_to_ast;

    fn dog_model_custom_directive_test(test_fn: impl FnOnce(DirectiveDiffer<'_>)) {
        let previous = r#"
        model Dog {
            id Int @id

            @@customDirective(hasFur: true, animalType: "Mammal")
        }
        "#;
        let previous = parse_to_ast(previous).unwrap();
        let next = r#"
        model Dog {
            id Int @id

            @@customDirective(animalType: "Mammals", legs: 4)
        }
        "#;
        let next = parse_to_ast(next).unwrap();

        let differ = TopDiffer {
            previous: &previous,
            next: &next,
        };

        let dog_diff: ModelDiffer<'_> = differ.model_pairs().next().unwrap();
        let custom_directive = dog_diff.directive_pairs().next().unwrap();

        assert_eq!(custom_directive.previous.name.name, "customDirective");

        test_fn(custom_directive)
    }

    #[test]
    fn datamodel_differ_directive_differ_works() {
        dog_model_custom_directive_test(|directive_diff| {
            let deleted_arguments = directive_diff.deleted_arguments().collect::<Vec<_>>();

            assert_eq!(deleted_arguments.len(), 1);
            assert_eq!(deleted_arguments.get(0).unwrap().name.name, "hasFur");

            let created_arguments = directive_diff.created_arguments().collect::<Vec<_>>();

            assert_eq!(created_arguments.len(), 1);
            assert_eq!(created_arguments.get(0).unwrap().name.name, "legs");
        })
    }
}
