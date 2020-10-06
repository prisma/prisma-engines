use super::{
    attributes::{attributes_are_identical, attributes_match, AttributeDiffer},
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
/// Attributes diffing on a model has to make a distinction between _repeated_ attributes and regular attributes.
/// Repeated attributes are attributes that can appear multiple times in the same model definition, like `@@unique`.
/// Most attributes can appear only once, so we call them regular attributes.
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

    /// Regular model attributes (`@@`) created in `next`.
    pub(crate) fn created_regular_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next_regular_attributes().filter(move |next_attribute| {
            self.previous_regular_attributes()
                .find(|previous_attribute| attributes_match(previous_attribute, next_attribute))
                .is_none()
        })
    }

    /// Regular model attributes (`@@`) deleted in `next`.
    pub(crate) fn deleted_regular_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous_regular_attributes().filter(move |previous_attribute| {
            self.next_regular_attributes()
                .find(|next_attribute| attributes_match(previous_attribute, next_attribute))
                .is_none()
        })
    }

    /// Iterator over the regular model attributes (`@@`) present in both `previous` and `next`.
    pub(crate) fn regular_attribute_pairs(&self) -> impl Iterator<Item = AttributeDiffer<'_>> {
        self.previous_regular_attributes()
            .filter_map(move |previous_attribute| {
                self.next_regular_attributes()
                    .find(|next_attribute| attributes_match(previous_attribute, next_attribute))
                    .map(|next_attribute| AttributeDiffer {
                        previous: previous_attribute,
                        next: next_attribute,
                    })
            })
    }

    pub(crate) fn created_repeated_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next_repeated_attributes().filter(move |next_attribute| {
            self.previous_repeated_attributes()
                .find(|previous_attribute| attributes_are_identical(previous_attribute, next_attribute))
                .is_none()
        })
    }

    pub(crate) fn deleted_repeated_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous_repeated_attributes().filter(move |previous_attribute| {
            self.next_repeated_attributes()
                .find(|next_attribute| attributes_are_identical(previous_attribute, next_attribute))
                .is_none()
        })
    }

    fn previous_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.previous.fields.iter()
    }

    fn next_fields(&self) -> impl Iterator<Item = &ast::Field> {
        self.next.fields.iter()
    }

    fn previous_regular_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous.attributes.iter().filter(attribute_is_regular)
    }

    fn next_regular_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next.attributes.iter().filter(attribute_is_regular)
    }

    fn previous_repeated_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous.attributes.iter().filter(attribute_is_repeated)
    }

    fn next_repeated_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next.attributes.iter().filter(attribute_is_repeated)
    }
}

fn fields_match(previous: &ast::Field, next: &ast::Field) -> bool {
    previous.name.name == next.name.name
}

/// Model attributes that can appear multiple times on the same model. Unlike others, they cannot be matched based only on the attribute name.
const REPEATED_MODEL_ATTRIBUTES: &[&str] = &["unique", "index"];

/// See ModelDiffer docs.
pub(super) fn attribute_is_regular(attribute: &&ast::Attribute) -> bool {
    !attribute_is_repeated(attribute)
}

/// See ModelDiffer docs.
pub(super) fn attribute_is_repeated(attribute: &&ast::Attribute) -> bool {
    REPEATED_MODEL_ATTRIBUTES.contains(&attribute.name.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::super::TopDiffer;
    use super::*;
    use datamodel::ast::parser::parse_schema;

    fn dog_datamodels_test(test_fn: impl FnOnce(ModelDiffer<'_>)) {
        let previous = r#"
        model Dog {
            id Int @id
            name String
            coat CoatCharacteristic[]
            isGoodDog Boolean

            @@customAttribute(hasFur: true)
            @@unique([name, coat])
        }

        enum CoatCharacteristic {
            Long
            Short
            Curly
        }
        "#;
        let previous = parse_schema(previous).unwrap();
        let next = r#"
        model Dog {
            id Int @id
            name String
            weight Float
            isGoodDog Boolean // always true

            @@map("goodDogs")
            @@customAttribute(hasFur: "Most of the time")
        }
        "#;
        let next = parse_schema(next).unwrap();

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
    fn datamodel_differ_model_differ_attribute_methods_work() {
        dog_datamodels_test(|model_diff| {
            let created_attributes: Vec<&ast::Attribute> = model_diff.created_regular_attributes().collect();

            assert_eq!(created_attributes.len(), 1);
            let created_attribute = created_attributes[0];
            assert_eq!(created_attribute.name.name, "map");
            assert_eq!(created_attribute.arguments.len(), 1);

            let deleted_attributes: Vec<&ast::Attribute> = model_diff.deleted_repeated_attributes().collect();

            assert_eq!(deleted_attributes.len(), 1);
            let deleted_attribute = deleted_attributes[0];
            assert_eq!(deleted_attribute.name.name, "unique");
            assert_eq!(deleted_attribute.arguments.len(), 1);

            assert_ne!(
                created_attribute
                    .arguments
                    .get(0)
                    .as_ref()
                    .unwrap()
                    .value
                    .render_to_string(),
                deleted_attribute
                    .arguments
                    .get(0)
                    .as_ref()
                    .unwrap()
                    .value
                    .render_to_string()
            );

            let attribute_pairs: Vec<_> = model_diff.regular_attribute_pairs().collect();

            assert_eq!(attribute_pairs.len(), 1);
            let first_attribute = attribute_pairs.get(0).unwrap();
            assert_eq!(first_attribute.previous.name.name, "customAttribute");
            assert_eq!(first_attribute.previous.name.name, first_attribute.next.name.name)
        });
    }

    #[test]
    fn datamodel_differ_model_differ_works_with_multiple_unique_attributes() {
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
        let previous = parse_schema(previous).unwrap();
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
        let next = parse_schema(next).unwrap();

        let top_differ = TopDiffer {
            previous: &previous,
            next: &next,
        };
        let model_differ = top_differ.model_pairs().next().unwrap();

        let created_regular_attribute_names: Vec<&String> = model_differ
            .created_regular_attributes()
            .map(|attribute| &attribute.name.name)
            .collect();
        let deleted_regular_attribute_names: Vec<&String> = model_differ
            .deleted_regular_attributes()
            .map(|attribute| &attribute.name.name)
            .collect();
        let updated_regular_attribute_names: Vec<&String> = model_differ
            .regular_attribute_pairs()
            .map(|attribute| &attribute.previous.name.name)
            .collect();

        assert_eq!(updated_regular_attribute_names, &["map"]);
        assert!(created_regular_attribute_names.is_empty());
        assert!(deleted_regular_attribute_names.is_empty());

        let created_repeated_attribute_names: Vec<_> = model_differ
            .created_repeated_attributes()
            .map(|attribute| &attribute.name.name)
            .collect();
        let deleted_repeated_attribute_names: Vec<_> = model_differ
            .deleted_repeated_attributes()
            .map(|attribute| &attribute.name.name)
            .collect();

        assert_eq!(created_repeated_attribute_names, &["unique", "unique"]);
        assert_eq!(deleted_repeated_attribute_names, &["unique", "unique"]);
    }
}
