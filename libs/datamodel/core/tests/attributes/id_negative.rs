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

    errors.assert_are(&[DatamodelError::new_attribute_validation_error(
        "Fields that are marked as id must be required.",
        "id",
        Span::new(36, 38),
    )]);
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
        Span::new(61, 66),
    ));
}

#[test]
fn relation_fields_as_part_of_compound_id_must_error() {
    let dml = r#"
    model User {
        name           String
        identification Identification @relation(references:[id])

        @@id([name, identification])
    }

    model Identification {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The id definition refers to the relation fields identification. ID definitions must reference only scalar fields.",
        "User",
        Span::new(124, 150),
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
        "The id definition refers to the optional fields b, c. ID definitions must reference only required fields.",
        "Model",
        Span::new(75, 86),
    ));
}

#[test]
fn stringified_field_names_in_id_return_nice_error() {
    let dm = r#"
        model User {
            firstName String
            lastName  String

            @@id(["firstName", "lastName"])
        }
    "#;

    let err = parse_error(dm);

    err.assert_is(DatamodelError::TypeMismatchError {
        expected_type: "constant literal".into(),
        received_type: "string".into(),
        raw: "firstName".into(),
        span: Span::new(99, 110),
    });
}

#[test]
fn relation_field_as_id_must_error() {
    let dml = r#"
    model User {
        identification Identification @relation(references:[id]) @id
    }

    model Identification {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The field `identification` is a relation field and cannot be marked with `@id`. Only scalar fields can be declared as id.",
        "id",
        Span::new(84, 86),
    ));
}

#[test]
fn unique_should_error_if_unique_and_id_are_specified_on_sqlserver() {
    let dml = r#"
    datasource test {
         provider = "sqlserver"
         url = "sqlserver://...."
    }
    
     model Model {
         id Int @id @unique
     }
     "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "SQLServer only allows either a unique or a primary key constraint on a column.",
        "unique",
        Span::new(140, 146),
    ));
}
