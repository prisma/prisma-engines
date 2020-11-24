use crate::common::*;
use datamodel::{ast::Span, diagnostics::DatamodelError};

#[test]
fn id_should_error_if_the_field_is_not_required() {
    let dml = r#"
    model Model {
        id Int? @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Fields that are marked as id must be required.",
        "id",
        Span::new(36, 38),
    ));
}

#[test]
fn id_should_error_if_unique_and_id_are_specified() {
    let dml = r#"
    model Model {
        id Int @id @unique
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Fields that are marked as id should not have an additional @unique.",
        "unique",
        Span::new(39, 45),
    ));
}

#[test]
fn id_should_error_multiple_ids_are_provided() {
    let dml = r#"
    model Model {
        id         Int      @id
        internalId String   @id @default(uuid())
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "At most one field must be marked as the id field with the `@id` attribute.",
        "Model",
        Span::new(5, 105),
    ));
}

#[test]
fn id_must_error_when_single_and_multi_field_id_is_used() {
    let dml = r#"
    model Model {
        id         Int      @id
        b          String

        @@id([id,b])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at most one id criteria. You can\'t have `@id` and `@@id` at the same time.",
        "Model",
        Span::new(5, 104),
    ));
}

#[test]
fn id_must_error_when_multi_field_is_referring_to_undefined_fields() {
    let dml = r#"
    model Model {
      a String
      b String

      @@id([a,c])
    }
    "#;
    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "The multi field id declaration refers to the unknown fields c.",
        "Model",
        Span::new(58, 67),
    ));
}

#[test]
fn must_error_when_multi_field_is_referring_fields_that_are_not_required() {
    let dml = r#"
    model Model {
      a String
      b String?
      c String?

      @@id([a,b,c])
    }
    "#;
    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "The id definition refers to the optional fields b, c. Id definitions must reference only required fields.",
        "Model",
        Span::new(75, 86),
    ));
}
