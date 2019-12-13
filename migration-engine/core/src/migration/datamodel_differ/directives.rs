use datamodel::ast;
use migration_connector::steps::MigrationExpression;

#[derive(Debug)]
pub(crate) struct DirectiveDiffer<'a> {
    pub(crate) previous: &'a ast::Directive,
    pub(crate) next: &'a ast::Directive,
}

impl<'a> DirectiveDiffer<'a> {
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

    fn previous_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.previous.arguments.iter()
    }

    fn next_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.next.arguments.iter()
    }
}

pub(crate) fn directives_match(previous: &ast::Directive, next: &ast::Directive) -> bool {
    previous.name.name == next.name.name
}

pub fn arguments_match(previous: &ast::Argument, next: &ast::Argument) -> bool {
    previous.name.name == next.name.name
}

pub(crate) fn directives_are_identical(previous: &ast::Directive, next: &ast::Directive) -> bool {
    if previous.name.name != next.name.name {
        return false;
    }

    if previous.arguments.len() != next.arguments.len() {
        return false;
    }

    previous.arguments.iter().all(move |previous_argument| {
        next.arguments
            .iter()
            .find(|next_argument| arguments_are_identical(previous_argument, next_argument))
            .is_some()
    })
}

fn arguments_are_identical(previous: &ast::Argument, next: &ast::Argument) -> bool {
    previous.name.name == next.name.name
        && MigrationExpression::from_ast_expression(&previous.value)
            == MigrationExpression::from_ast_expression(&next.value)
}

#[cfg(test)]
mod tests {
    use super::super::{ModelDiffer, TopDiffer};
    use super::*;
    use datamodel::ast::parser::parse;

    fn dog_model_custom_directive_test(test_fn: impl FnOnce(DirectiveDiffer<'_>)) {
        let previous = r#"
        model Dog {
            id Int @id

            @@customDirective(hasFur: true, animalType: "Mammal")
        }
        "#;
        let previous = parse(previous).unwrap();
        let next = r#"
        model Dog {
            id Int @id

            @@customDirective(animalType: "Mammals", legs: 4)
        }
        "#;
        let next = parse(next).unwrap();

        let differ = TopDiffer {
            previous: &previous,
            next: &next,
        };

        let dog_diff: ModelDiffer<'_> = differ.model_pairs().next().unwrap();
        let custom_directive = dog_diff.regular_directive_pairs().next().unwrap();

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
