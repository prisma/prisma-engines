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

    errors.assert_are(&[
        DatamodelError::new_attribute_validation_error(
            "Fields that are marked as id must be required.",
            "id",
            Span::new(36, 38),
        ),
        DatamodelError::new_model_validation_error(
            "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model. The following unique criterias were not considered as they contain fields that are not required:\n- id",
            "Model",
            Span::new(5, 44),
        ),
    ]);
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
        Span::new(88, 98),
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
        "The id definition refers to the relation fields identification. Id definitions must reference only scalar fields.",
        "User",
        Span::new(136, 162),
    ));
}

#[test]
fn invalid_name_for_compound_id_must_error() {
    let dml = r#"
    model User {
        name           String            
        identification Int

        @@id([name, identification], name: "Test.User")
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The `name` property within the `@@id` attribute only allows for the following characters: `_a-zA-Z0-9`.",
        "User",
        Span::new(98, 143),
    ));
}

#[test]
fn mapped_id_must_error_on_mysql() {
    let dml = r#"
    datasource test {
        provider = "mysql"
        url = "mysql://root:prisma@127.0.0.1:3309/NoNamedPKsOnMysql"
    }
    
    model User {
        name           String            
        identification Int

        @@id([name, identification], map: "NotSupportedByProvider")
    }
    
    model User1 {
        name           String @id("NotSupportedByProvider")
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_are(&[
        DatamodelError::new_model_validation_error(
            "You defined a database name for the primary key on the model. This is not supported by the provider.",
            "User",
            Span::new(134, 290),
        ),
        DatamodelError::new_model_validation_error(
            "You defined a database name for the primary key on the model. This is not supported by the provider.",
            "User1",
            Span::new(300, 379),
        ),
    ]);
}

#[test]
fn mapped_id_must_error_on_sqlite() {
    let dml = r#"
    datasource test {
        provider = "sqlite"
        url = "file://...."
    }
    
   model User {
        name           String            
        identification Int

        @@id([name, identification], map: "NotSupportedByProvider")
    }
    
    model User1 {
        name           String @id("NotSupportedByProvider")
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_are(&[
        DatamodelError::new_model_validation_error(
            "You defined a database name for the primary key on the model. This is not supported by the provider.",
            "User",
            Span::new(93, 249),
        ),
        DatamodelError::new_model_validation_error(
            "You defined a database name for the primary key on the model. This is not supported by the provider.",
            "User1",
            Span::new(259, 338),
        ),
    ]);
}

#[test]
fn naming_id_to_a_field_name_should_error() {
    let dml = r#"
    model User {
        used           Int
        name           String            
        identification Int

        @@id([name, identification], name: "used")
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The custom name specified for the `@@id` attribute is already used as a name for a field. Please choose a different name.",
        "User",
        Span::new(5, 171),
    ));
}

#[test]
fn mapping_id_with_a_name_that_is_too_long_should_error() {
    let dml = r#"
    datasource test {
        provider = "postgresql"
        url = "postgresql://root:prisma@127.0.0.1:3309/postgres"
    }
    
    model User {
        name           String            
        identification Int

        @@id([name, identification], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")
    }
    
    model User1 {
        name           String @id("IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell")            
        identification Int      
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_are(&[
        DatamodelError::new_model_validation_error(
            "The name specified for the `id` constraint `IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits` is too long for your chosen provider. The maximum allowed length is 63 bytes.",
            "User",
            Span::new(135, 388),
        ),
        DatamodelError::new_model_validation_error(
            "The name specified for the `id` constraint `IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell` is too long for your chosen provider. The maximum allowed length is 63 bytes.",
            "User1",
            Span::new(398, 629),
        ),
    ]);
}
