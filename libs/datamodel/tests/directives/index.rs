use datamodel::{ast::Span, errors::*, render_to_string, IndexDefinition, IndexType};

use crate::common::*;

#[test]
fn basic_index_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@index([firstName,lastName])
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
    });
}

#[test]
fn the_name_argument_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@index([firstName,lastName], name: "MyIndexName")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
    });
}

#[test]
fn multiple_index_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@index([firstName,lastName])
        @@index([firstName,lastName], name: "MyIndexName")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");

    user_model.assert_has_index(IndexDefinition {
        name: None,
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
    });

    user_model.assert_has_index(IndexDefinition {
        name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
    });
}

#[test]
fn must_error_when_unknown_fields_are_used() {
    let dml = r#"
    model User {
        id Int @id

        @@index([foo,bar])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(ValidationError::new_model_validation_error(
        "The index definition refers to the unknown fields foo, bar.",
        "User",
        Span::new(48, 64),
    ));
}

#[test]
fn index_directives_must_serialize_to_valid_dml() {
    let dml = r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@index([firstName,lastName], name: "customName")
        }
    "#;
    let schema = parse(dml);

    assert!(datamodel::parse(&render_to_string(&schema).unwrap()).is_ok());
}
