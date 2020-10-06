use crate::common::*;
use datamodel::ast::Span;
use datamodel::error::DatamodelError;

#[test]
fn map_attribute() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @map("first_name")

        @@map("user")
    }

    model Post {
        id Int @id
        text String @map(name: "post_text")

        @@map(name: "posti")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User").assert_with_db_name("user");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_with_db_name("first_name");

    let post_model = schema.assert_has_model("Post").assert_with_db_name("posti");
    post_model
        .assert_has_scalar_field("text")
        .assert_with_db_name("post_text");
}

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
        "The attribute `@map` can not be used on relation fields.",
        "map",
        Span::new(128, 146),
    ));
}
