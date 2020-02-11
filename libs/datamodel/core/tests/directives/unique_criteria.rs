use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError};

#[test]
fn must_error_on_model_without_unique_criteria() {
    let dml = r#"
    model Model {
        id String
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at least one unique criteria. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.",
        "Model",
        Span::new(5, 42),
    ));
}

#[test]
fn must_succeed_on_model_with_unique_criteria() {
    let dml1 = r#"
    model Model {
        id String @id
    }
    "#;
    let _ = parse(dml1);

    let dml2 = r#"
    model Model {
        a String
        b String
        @@id([a,b])
    }
    "#;
    let _ = parse(dml2);

    let dml3 = r#"
    model Model {
        unique String @unique
    }
    "#;
    let _ = parse(dml3);

    let dml4 = r#"
    model Model {
        a String
        b String
        @@unique([a,b])
    }
    "#;
    let _ = parse(dml4);
}
