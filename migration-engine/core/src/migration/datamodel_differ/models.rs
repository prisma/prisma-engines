use super::{directives::DirectiveDiffer, values::values_match, FieldDiffer};
use datamodel::ast;

/// Implements the logic to diff a pair of [AST models](/datamodel/ast/struct.Model.html).
pub(crate) struct ModelDiffer<'a> {
    pub(crate) previous: &'a ast::Model,
    pub(crate) next: &'a ast::Model,
}

impl<'a> ModelDiffer<'a> {
    fn previous_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.previous.fields.iter()
    }

    fn next_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.next.fields.iter()
    }

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

    fn previous_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter()
    }

    fn next_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next.directives.iter()
    }

    /// Model directives (`@@`) created in `next`.
    pub(crate) fn created_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_directives().filter(move |next_directive| {
            self.previous_directives()
                .find(|previous_directive| model_directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Model directives (`@@`) deleted in `next`.
    pub(crate) fn deleted_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_directives().filter(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| model_directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Iterator over the model directives (@@) present in both `previous` and `next`.
    pub(crate) fn directive_pairs(&self) -> impl Iterator<Item = DirectiveDiffer<'_>> {
        self.previous_directives().filter_map(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| model_directives_match(previous_directive, next_directive))
                .map(|next_directive| DirectiveDiffer {
                    previous: previous_directive,
                    next: next_directive,
                })
        })
    }
}

fn fields_match(previous: &ast::Field, next: &ast::Field) -> bool {
    previous.name.name == next.name.name
}

fn model_directives_match(previous: &ast::Directive, next: &ast::Directive) -> bool {
    if previous.name.name != next.name.name {
        return false;
    }

    if ["unique", "index"].contains(&previous.name.name.as_str()) {
        // TODO: implement fine grained index diffing
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::super::{directives::directives_match_exactly, TopDiffer};
    use super::*;
    use datamodel::parse_to_ast;

    fn dog_datamodels_test(test_fn: impl FnOnce(ModelDiffer<'_>)) {
        let previous = r#"
        model Dog {
            id Int @id
            name String
            coat CoatCharacteristic[]
            isGoodDog Boolean

            @@customDirective(hasFur: true)
            @@map("goodDogs")
            @@unique([name, coat])
        }

        enum CoatCharacteristic {
            Long
            Short
            Curly
        }
        "#;
        let previous = parse_to_ast(previous).unwrap();
        let next = r#"
        model Dog {
            id Int @id
            name String
            weight Float
            isGoodDog Boolean // always true

            @@unique([name, weight])
            @@map("goodDogs")
            @@customDirective(hasFur: "Most of the time")
        }
        "#;
        let next = parse_to_ast(next).unwrap();

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
            let created_directives: Vec<&ast::Directive> = model_diff.created_directives().collect();

            assert_eq!(created_directives.len(), 1);
            let created_directive = created_directives[0];
            assert_eq!(created_directive.name.name, "unique");
            assert_eq!(created_directive.arguments.len(), 1);

            let deleted_directives: Vec<&ast::Directive> = model_diff.deleted_directives().collect();

            assert_eq!(deleted_directives.len(), 1);
            let deleted_directive = deleted_directives[0];
            assert_eq!(deleted_directive.name.name, "unique");
            assert_eq!(deleted_directive.arguments.len(), 1);

            assert!(!values_match(
                &created_directive.arguments.get(0).as_ref().unwrap().value,
                &deleted_directive.arguments.get(0).as_ref().unwrap().value
            ));

            let directive_pairs: Vec<_> = model_diff
                .directive_pairs()
                .filter(|differ| !directives_match_exactly(differ.previous, differ.next))
                .collect();

            assert_eq!(directive_pairs.len(), 1);
            let first_directive = directive_pairs.get(0).unwrap();
            assert_eq!(first_directive.previous.name.name, "customDirective");
            assert_eq!(first_directive.previous.name.name, first_directive.next.name.name)
        });
    }
}
