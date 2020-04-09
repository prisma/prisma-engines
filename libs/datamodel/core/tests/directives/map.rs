use crate::common::*;
use datamodel::ast::Span;
use datamodel::{ast, error::DatamodelError};

#[test]
fn map_directive() {
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
        .assert_has_field("firstName")
        .assert_with_db_name("first_name");

    let post_model = schema.assert_has_model("Post").assert_with_db_name("posti");
    post_model.assert_has_field("text").assert_with_db_name("post_text");
}

#[test]
#[ignore]
fn map_must_fail_on_multiple_args_for_enums() {
    let dml = r#"
    enum Status {
        A
        B
        
        @@map(["name1", "name2"])
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is_at(
        0,
        DatamodelError::new_directive_validation_error(
            "A Model must not specify multiple mapped names.",
            "map",
            ast::Span::new(56, 79),
        ),
    );
}

#[test]
#[ignore] // this is hard to implement with the current abstraction in use for `@map`
fn map_must_error_for_relation_fields() {
    let dml = r#"
    model User {
        id Int @id
        fooId Int
        relationField  Foo @relation(fields: [fooId], references: [id]) @map(["custom_name"])
    }
    
    model Foo {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_directive_validation_error(
        "",
        "map",
        Span::new(0, 0),
    ));
}
