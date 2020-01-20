use crate::common::*;
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
fn map_must_fail_on_multiple_args_for_scalar_fields() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @map(["name1", "name2"])
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is_at(
        0,
        DatamodelError::new_directive_validation_error(
            "A scalar Field must not specify multiple mapped names.",
            "map",
            ast::Span::new(63, 86),
        ),
    );
}

#[test]
fn map_must_fail_on_multiple_args_for_models() {
    let dml = r#"
    model User {
        id Int @id
        
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
fn map_must_fail_on_wrong_number_of_args_for_relation_fields() {
    let dml = r#"
    model User {
        id Int @id
        firstRelation Foo @map(["name1", "name2"]) @relation("One", references: id)
        secondRelation Foo @map(["name1"]) @relation("Two", references:[a,b])
    }
    
    model Foo {
        id Int @id
        a String
        b String
        userOne User @relation("One")
        userTwo User @relation("Two")
        
        @@unique([a,b])
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is_at(
        0,
        DatamodelError::new_directive_validation_error(
            "This Relation Field must specify exactly 1 mapped names.",
            "map",
            ast::Span::new(64, 87),
        ),
    );
    errors.assert_is_at(
        1,
        DatamodelError::new_directive_validation_error(
            "This Relation Field must specify exactly 2 mapped names.",
            "map",
            ast::Span::new(149, 163),
        ),
    );
}

#[test]
fn map_must_work_on_right_number_of_args_for_relation_fields() {
    let dml = r#"
    model User {
        id Int @id
        firstRelation Foo @map(["name1"]) @relation(name: "One")
        secondRelation Foo @map(["name1", "name2"]) @relation(name: "Two", references:[a,b])
    }
    
    model Foo {
        id Int @id
        a String
        b String
        
        @@unique([a,b])
    }
    "#;

    parse(dml);
}
