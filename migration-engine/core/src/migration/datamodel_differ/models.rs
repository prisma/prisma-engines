use super::{
    directives::{directives_are_identical, directives_match, DirectiveDiffer},
    FieldDiffer,
};
use datamodel::ast;

/// Implements the logic to diff a pair of [AST models](/datamodel/ast/struct.Model.html).
#[derive(Debug)]
pub(crate) struct ModelDiffer<'a> {
    pub(crate) previous: &'a ast::Model,
    pub(crate) next: &'a ast::Model,
}

/// Diff two models in a schema AST.
///
/// Directives diffing on a model has to make a distinction between _repeated_ directives and regular directives.
/// Repeated directives are directives that can appear multiple times in the same model definition, like `@@unique`.
/// Most directives can appear only once, so we call them regular directives.
impl<'a> ModelDiffer<'a> {
    /// Iterator over the fields present in `next` but not `previous`.
    pub(crate) fn created_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.next_fields().filter(move |next_field| {
            self.previous_fields()
                .find(|previous_field| fields_match(previous_field, next_field))
                .is_none()
        })
    }

    /// Iterator over the fields present in `previous` but not `next`.
    pub(crate) fn deleted_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.previous_fields().filter(move |previous_field| {
            self.next_fields()
                .find(|next_field| fields_match(previous_field, next_field))
                .is_none()
        })
    }

    /// Iterator over the fields present in both `previous` and `next`.
    pub(crate) fn field_pairs(&self) -> impl Iterator<Item = FieldDiffer<'_>> {
        self.previous_fields().filter_map(move |previous_field| {
            self.next_fields()
                .find(|next_field| fields_match(previous_field, next_field))
                .map(|next_field| FieldDiffer {
                    previous: previous_field,
                    next: next_field,
                })
        })
    }

    /// Regular model directives (`@@`) created in `next`.
    pub(crate) fn created_regular_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_regular_directives().filter(move |next_directive| {
            self.previous_regular_directives()
                .find(|previous_directive| directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Regular model directives (`@@`) deleted in `next`.
    pub(crate) fn deleted_regular_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_regular_directives().filter(move |previous_directive| {
            self.next_regular_directives()
                .find(|next_directive| directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Iterator over the regular model directives (`@@`) present in both `previous` and `next`.
    pub(crate) fn regular_directive_pairs(&self) -> impl Iterator<Item = DirectiveDiffer<'_>> {
        self.previous_regular_directives()
            .filter_map(move |previous_directive| {
                self.next_regular_directives()
                    .find(|next_directive| directives_match(previous_directive, next_directive))
                    .map(|next_directive| DirectiveDiffer {
                        previous: previous_directive,
                        next: next_directive,
                    })
            })
    }

    pub(crate) fn created_repeated_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_repeated_directives().filter(move |next_directive| {
            self.previous_repeated_directives()
                .find(|previous_directive| directives_are_identical(previous_directive, next_directive))
                .is_none()
        })
    }

    pub(crate) fn deleted_repeated_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_repeated_directives().filter(move |previous_directive| {
            self.next_repeated_directives()
                .find(|next_directive| directives_are_identical(previous_directive, next_directive))
                .is_none()
        })
    }

    fn previous_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.previous.fields.iter()
    }

    fn next_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.next.fields.iter()
    }

    fn previous_regular_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter().filter(is_regular)
    }

    fn next_regular_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next.directives.iter().filter(is_regular)
    }

    fn previous_repeated_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter().filter(is_repeated)
    }

    fn next_repeated_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next.directives.iter().filter(is_repeated)
    }
}

fn fields_match(previous: &ast::Field, next: &ast::Field) -> bool {
    previous.name.name == next.name.name
}

/// Model directives that can appear multiple times on the same model. Unlike others, they cannot be matched based only on the directive name.
const REPEATED_MODEL_DIRECTIVES: &[&str] = &["unique", "index"];

/// See ModelDiffer docs.
fn is_regular(directive: &&ast::Directive) -> bool {
    !is_repeated(directive)
}

/// See ModelDiffer docs.
fn is_repeated(directive: &&ast::Directive) -> bool {
    REPEATED_MODEL_DIRECTIVES.contains(&directive.name.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::super::TopDiffer;
    use super::*;
    use datamodel::ast::parser::parse;

    fn dog_datamodels_test(test_fn: impl FnOnce(ModelDiffer<'_>)) {
        let previous = r#"
        model Dog {
            id Int @id
            name String
            coat CoatCharacteristic[]
            isGoodDog Boolean

            @@customDirective(hasFur: true)
            @@unique([name, coat])
        }

        enum CoatCharacteristic {
            Long
            Short
            Curly
        }
        "#;
        let previous = parse(previous).unwrap();
        let next = r#"
        model Dog {
            id Int @id
            name String
            weight Float
            isGoodDog Boolean // always true

            @@map("goodDogs")
            @@customDirective(hasFur: "Most of the time")
        }
        "#;
        let next = parse(next).unwrap();

        let top_differ = TopDiffer {
            previous: &previous,
            next: &next,
        };
        let model_differ = top_differ.model_pairs().next().unwrap();

        test_fn(model_differ)
    }

    #[test]
    fn datamodel_differ_model_differ_field_methods_work() {
        dog_datamodels_test(|model_diff| {
            let created_fields: Vec<&str> = model_diff
                .created_fields()
                .map(|field| field.name.name.as_str())
                .collect();
            assert_eq!(created_fields, &["weight"]);

            let deleted_fields: Vec<&str> = model_diff
                .deleted_fields()
                .map(|field| field.name.name.as_str())
                .collect();
            assert_eq!(deleted_fields, &["coat"]);

            let field_pairs: Vec<(&str, &str)> = model_diff
                .field_pairs()
                .map(|differ| (differ.previous.name.name.as_str(), differ.next.name.name.as_str()))
                .collect();
            assert_eq!(
                field_pairs,
                &[("id", "id"), ("name", "name"), ("isGoodDog", "isGoodDog")]
            );
        })
    }

    #[test]
    fn datamodel_differ_model_differ_directive_methods_work() {
        dog_datamodels_test(|model_diff| {
            let created_directives: Vec<&ast::Directive> = model_diff.created_regular_directives().collect();

            assert_eq!(created_directives.len(), 1);
            let created_directive = created_directives[0];
            assert_eq!(created_directive.name.name, "map");
            assert_eq!(created_directive.arguments.len(), 1);

            let deleted_directives: Vec<&ast::Directive> = model_diff.deleted_repeated_directives().collect();

            assert_eq!(deleted_directives.len(), 1);
            let deleted_directive = deleted_directives[0];
            assert_eq!(deleted_directive.name.name, "unique");
            assert_eq!(deleted_directive.arguments.len(), 1);

            assert_ne!(
                created_directive
                    .arguments
                    .get(0)
                    .as_ref()
                    .unwrap()
                    .value
                    .render_to_string(),
                deleted_directive
                    .arguments
                    .get(0)
                    .as_ref()
                    .unwrap()
                    .value
                    .render_to_string()
            );

            let directive_pairs: Vec<_> = model_diff.regular_directive_pairs().collect();

            assert_eq!(directive_pairs.len(), 1);
            let first_directive = directive_pairs.get(0).unwrap();
            assert_eq!(first_directive.previous.name.name, "customDirective");
            assert_eq!(first_directive.previous.name.name, first_directive.next.name.name)
        });
    }

    #[test]
    fn datamodel_differ_model_differ_works_with_multiple_unique_directives() {
        let previous = r#"
            model Test {
                id Int @id
                a String
                b String
                c String
                d String

                @@map("test_table")
                @@unique([a, b])
                @@unique([c, d])
            }
        "#;
        let previous = parse(previous).unwrap();
        let next = r#"
            model Test {
                id Int @id
                a String
                b String
                c String
                d String

                @@map("test_table")
                @@unique([a, d])
                @@unique([b, d])
            }
        "#;
        let next = parse(next).unwrap();

        let top_differ = TopDiffer {
            previous: &previous,
            next: &next,
        };
        let model_differ = top_differ.model_pairs().next().unwrap();

        let created_regular_directive_names: Vec<&String> = model_differ
            .created_regular_directives()
            .map(|directive| &directive.name.name)
            .collect();
        let deleted_regular_directive_names: Vec<&String> = model_differ
            .deleted_regular_directives()
            .map(|directive| &directive.name.name)
            .collect();
        let updated_regular_directive_names: Vec<&String> = model_differ
            .regular_directive_pairs()
            .map(|directive| &directive.previous.name.name)
            .collect();

        assert_eq!(updated_regular_directive_names, &["map"]);
        assert!(created_regular_directive_names.is_empty());
        assert!(deleted_regular_directive_names.is_empty());

        let created_repeated_directive_names: Vec<_> = model_differ
            .created_repeated_directives()
            .map(|directive| &directive.name.name)
            .collect();
        let deleted_repeated_directive_names: Vec<_> = model_differ
            .deleted_repeated_directives()
            .map(|directive| &directive.name.name)
            .collect();

        assert_eq!(created_repeated_directive_names, &["unique", "unique"]);
        assert_eq!(deleted_repeated_directive_names, &["unique", "unique"]);
    }
}
