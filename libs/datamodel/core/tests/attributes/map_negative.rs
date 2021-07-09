use crate::common::*;
use datamodel::ast::Span;
use datamodel::diagnostics::DatamodelError;

#[test]
fn map_must_error_for_relation_fields() {
    let dml = r#"
    model User {
        id Int @id
        fooId Int
        relationField  Foo @relation(fields: [fooId], references: [id]) @map("custom_name")
    }

    model Foo {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The attribute `@map` cannot be used on relation fields.",
        "map",
        Span::new(128, 146),
    ));
}
