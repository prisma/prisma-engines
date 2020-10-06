use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError};

#[test]
fn fail_on_duplicate_attribute() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @map(name: "first_name", name: "Duplicate")
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_argument_error("name", Span::new(87, 104)));
}

#[test]
fn fail_on_duplicate_unnamed_attribute() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @map("first_name", name: "Duplicate")
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_default_argument_error(
        "name",
        Span::new(81, 98),
    ));
}

#[test]
fn fail_on_extra_argument() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @map("first_name", unused: "Unnamed")
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_unused_argument_error("unused", Span::new(81, 98)));
}
