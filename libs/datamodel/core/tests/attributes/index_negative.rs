use datamodel::{ast::Span, diagnostics::*};

use crate::common::*;

#[test]
fn indexes_on_relation_fields_must_error() {
    let dml = r#"
    model User {
        id               Int @id
        identificationId Int

        identification   Identification @relation(fields: [identificationId], references:[id])

        @@index([identification])
    }

    model Identification {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The index definition refers to the relation fields identification. Index definitions must reference only scalar fields. Did you mean `@@index([identificationId])`?",
        "User",
        Span::new(187,210),
    ));
}

#[test]
fn empty_index_names_are_rejected() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@index([firstName,lastName], name: "")
    }
    "#;

    let err = datamodel::parse_datamodel(dml).unwrap_err();

    err.assert_is(DatamodelError::AttributeValidationError {
        message: "The `name` argument cannot be an empty string.".into(),
        attribute_name: "index".into(),
        span: Span::new(108, 145),
    });
}

#[test]
fn empty_unique_index_names_are_rejected() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@unique([firstName,lastName], name: "")
    }
    "#;

    let err = datamodel::parse_datamodel(dml).unwrap_err();

    err.assert_is(DatamodelError::AttributeValidationError {
        message: "The `name` argument cannot be an empty string.".into(),
        attribute_name: "unique".into(),
        span: Span::new(108, 146),
    });
}

#[test]
fn multiple_indexes_with_same_name_are_not_supported_by_sqlite() {
    let dml = r#"
    datasource sqlite {
        provider = "sqlite"
        url = "sqlite://asdlj"
    }

    model User {
        id         Int @id
        neighborId Int

        @@index([id], name: "MyIndexName")
     }

     model Post {
        id Int @id
        optionId Int

        @@index([id], name: "MyIndexName")
     }
    "#;

    let errors = parse_error(dml);

    errors.assert_length(1);
    errors.assert_is_at(
        0,
        DatamodelError::new_multiple_indexes_with_same_name_are_not_supported("MyIndexName", Span::new(279, 311)),
    );
}

#[test]
fn multiple_indexes_with_same_name_are_not_supported_by_postgres() {
    let dml = r#"
    datasource postgres {
        provider = "postgres"
        url = "postgres://asdlj"
    }

    model User {
        id         Int @id
        neighborId Int

        @@index([id], name: "MyIndexName")
     }

     model Post {
        id Int @id
        optionId Int

        @@index([id], name: "MyIndexName")
     }
    "#;

    let errors = parse_error(dml);
    for error in errors.errors() {
        println!("DATAMODEL ERROR: {:?}", error);
    }

    errors.assert_length(1);
    errors.assert_is_at(
        0,
        DatamodelError::new_multiple_indexes_with_same_name_are_not_supported("MyIndexName", Span::new(285, 317)),
    );
}

#[test]
fn unique_insert_with_same_name_are_not_supported_by_postgres() {
    let dml = r#"
    datasource postgres {
        provider = "postgres"
        url = "postgres://asdlj"
    }

    model User {
        id         Int @id
        neighborId Int

        @@index([id], name: "MyIndexName")
     }

     model Post {
        id Int @id
        optionId Int

        @@unique([id], name: "MyIndexName")
     }
    "#;

    let errors = parse_error(dml);
    for error in errors.errors() {
        println!("DATAMODEL ERROR: {:?}", error);
    }

    errors.assert_length(1);
    errors.assert_is_at(
        0,
        DatamodelError::new_multiple_indexes_with_same_name_are_not_supported("MyIndexName", Span::new(285, 318)),
    );
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

    errors.assert_is(DatamodelError::new_model_validation_error(
        "The index definition refers to the unknown fields foo, bar.",
        "User",
        Span::new(48, 64),
    ));
}

#[test]
fn stringified_field_names_in_index_return_nice_error() {
    let dm = r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@index(["firstName", "lastName"])
        }
    "#;

    let err = parse_error(dm);

    err.assert_is(DatamodelError::TypeMismatchError {
        expected_type: "constant literal".into(),
        received_type: "string".into(),
        raw: "firstName".into(),
        span: Span::new(135, 146),
    });
}
