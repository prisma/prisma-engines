use crate::attributes::with_named_constraints;
use crate::common::*;
use datamodel::{ast::Span, diagnostics::DatamodelError};

#[test]
fn must_error_on_model_without_unique_criteria() {
    let dml = r#"
    model Model {
        id String
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.",
        "Model",
        Span::new(5, 42),
    ));
}

#[test]
fn must_error_if_only_loose_unique_criterias_are_present() {
    let dml = r#"
    model Model {
        id   String
        name String? @unique 
        a    String
        b    String?
        @@unique([a,b])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model. The following unique criterias were not considered as they contain fields that are not required:\n- name\n- a, b",
        "Model",
        Span::new(5, 139),
    ));
}

#[test]
fn multiple_unnamed_arguments_must_error() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@unique(firstName,lastName)
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_attribute_validation_error("You provided multiple unnamed arguments. This is not possible. Did you forget the brackets? Did you mean `[firstName, lastName]`?", "unique", Span::new(108, 134)));
}

#[test]
fn multi_field_unique_indexes_on_relation_fields_must_error_and_give_nice_error_on_inline_side() {
    let dml = r#"
    model User {
        id               Int @id
        identificationId Int
        identification Identification @relation(fields: [identificationId], references:[id])

        @@unique([identification])
    }

    model Identification {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error("The unique index definition refers to the relation fields identification. Index definitions must reference only scalar fields. Did you mean `@@unique([identificationId])`?", "User",Span::new(184, 208)));
}

#[test]
fn multi_field_unique_indexes_on_relation_fields_must_error_and_give_nice_error_on_non_inline_side() {
    let dml = r#"
    model User {
        id               Int @id
        identificationId Int
        identification   Identification @relation(fields: [identificationId], references:[id])
    }

    model Identification {
        id   Int @id
        user User
        @@unique([user])
    }
    "#;

    let errors = parse_error(dml);
    // in this case the error can't give a suggestion
    errors.assert_is(DatamodelError::new_model_validation_error("The unique index definition refers to the relation fields user. Index definitions must reference only scalar fields.", "Identification",Span::new(258, 272)));
}

#[test]
fn single_field_unique_on_relation_fields_must_error_nicely_with_one_underlying_fields() {
    let dml = r#"
    model User {
        id               Int @id
        identificationId Int
        identification Identification @relation(fields: [identificationId], references:[id]) @unique
    }

    model Identification {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_attribute_validation_error("The field `identification` is a relation field and cannot be marked with `unique`. Only scalar fields can be made unique. Did you mean to put it on `identificationId`?", "unique", Span::new(174, 180)));
}

#[test]
fn single_field_unique_on_relation_fields_must_error_nicely_with_many_underlying_fields() {
    let dml = r#"
    model User {
        id                Int @id
        identificationId1 Int
        identificationId2 Int
        identification Identification @relation(fields: [identificationId1, identificationId2], references:[id1, id2]) @unique
    }

    model Identification {
        id1 Int
        id2 Int
        @@id([id1, id2])
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_attribute_validation_error("The field `identification` is a relation field and cannot be marked with `unique`. Only scalar fields can be made unique. Did you mean to provide `@@unique([identificationId1, identificationId2])`?", "unique", Span::new(232, 238)));
}

#[test]
fn stringified_field_names_in_unique_return_nice_error() {
    let dm = r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@unique(["firstName", "lastName"])
        }
    "#;

    let err = parse_error(dm);

    err.assert_is(DatamodelError::TypeMismatchError {
        expected_type: "constant literal".into(),
        received_type: "string".into(),
        raw: "firstName".into(),
        span: Span::new(136, 147),
    });
}

#[test]
fn must_error_when_unknown_fields_are_used() {
    let dml = r#"
    model User {
        id Int @id

        @@unique([foo,bar])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "The unique index definition refers to the unknown fields foo, bar.",
        "User",
        Span::new(48, 65),
    ));
}

#[test]
fn must_error_when_using_the_same_field_multiple_times() {
    let dml = r#"
    model User {
        id    Int    @id
        email String @unique

        @@unique([email, email])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "The unique index definition refers to the field email multiple times.",
        "User",
        Span::new(83, 105),
    ));
}

#[test]
fn invalid_name_for_compound_unique_must_error() {
    let dml = r#"
     datasource test {
            provider = "mysql"
            url = "mysql://root:prisma@127.0.0.1:3309/ReproIndexNames?connection_limit=1"
     }
    
     generator js {
            provider = "prisma-client-js"
            previewFeatures = ["NamedConstraints"]
     }
     
     model User {
         name           String            
         identification Int

         @@unique([name, identification], name: "Test.User")
     }
     "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The `name` property within the `@@unique` attribute only allows for the following characters: `_a-zA-Z0-9`.",
        "User",
        Span::new(384, 433),
    ));
}

#[test]
fn mapping_unique_with_a_name_that_is_too_long_should_error() {
    let dml = r#"
     datasource test {
            provider = "mysql"
            url = "mysql://root:prisma@127.0.0.1:3309/ReproIndexNames?connection_limit=1"
     }
    
     generator js {
            provider = "prisma-client-js"
            previewFeatures = ["NamedConstraints"]
     }
     
     model User {
         name           String            
         identification Int

         @@unique([name, identification], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")
     }
     
     model User1 {
         name           String @unique(map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell")            
         identification Int      
     }
     "#;

    let errors = parse_error(dml);
    errors.assert_are(&[
        DatamodelError::new_model_validation_error(
            "The constraint name 'IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits' specified in the `map` argument for the `@@unique` constraint is too long for your chosen provider. The maximum allowed length is 64 bytes.",
            "User",
            Span::new(384, 542),
        ),
        DatamodelError::new_model_validation_error(
            "The constraint name 'IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell' specified in the `map` argument for the `@unique` constraint is too long for your chosen provider. The maximum allowed length is 64 bytes.",
            "User1",
            Span::new(607, 751),
        ),
    ]);
}

#[test]
fn naming_unique_to_a_field_name_should_error() {
    let dml = with_named_constraints(
        r#"
     model User {
         used           Int
         name           String            
         identification Int

         @@unique([name, identification], name: "used")
     }
     "#,
    );

    let errors = parse_error(&dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The custom name `used` specified for the `@@unique` attribute is already used as a name for a field. Please choose a different name.",
        "User",
        Span::new(229, 404),
    ));
}

#[test]
fn naming_field_level_unique_should_error() {
    let dml = with_named_constraints(
        r#"
     model User {
         used           Int @unique(name: "INVALID ON FIELD LEVEL")
     }
     "#,
    );

    let errors = parse_error(&dml);
    errors.assert_is(DatamodelError::new_unused_argument_error("name", Span::new(278, 308)));
}

#[test]
fn duplicate_implicit_names_should_error() {
    let dml = with_named_constraints(
        r#"
     model User {
         used           Int @unique
         
         @@unique([used])
     }
     "#,
    );

    let errors = parse_error(&dml);
    errors.assert_is(DatamodelError::new_multiple_indexes_with_same_name_are_not_supported(
        "User_used_key",
        Span::new(299, 313),
    ));
}
