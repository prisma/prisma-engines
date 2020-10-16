use datamodel::{ast::Span, diagnostics::*, render_datamodel_to_string, IndexDefinition, IndexType};

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
fn indexes_on_enum_fields_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        role      Role

        @@index([role])
    }
    
    enum Role {
        Admin
        Member
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        fields: vec!["role".to_string()],
        tpe: IndexType::Normal,
    });
}

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
        Span::new(195,218),
    ));
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
fn multiple_indexes_with_same_name_are_supported_by_mysql() {
    let dml = r#"
    datasource mysql {
        provider = "mysql"
        url = "mysql://asdlj"
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

    let schema = parse(dml);

    let user_model = schema.assert_has_model("User");
    let post_model = schema.assert_has_model("Post");

    user_model.assert_has_index(IndexDefinition {
        name: Some("MyIndexName".to_string()),
        fields: vec!["id".to_string()],
        tpe: IndexType::Normal,
    });

    post_model.assert_has_index(IndexDefinition {
        name: Some("MyIndexName".to_string()),
        fields: vec!["id".to_string()],
        tpe: IndexType::Normal,
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
    for error in errors.errors.iter() {
        println!("DATAMODEL ERROR: {:?}", error);
    }

    errors.assert_length(1);
    errors.assert_is_at(
        0,
        DatamodelError::new_multiple_indexes_with_same_name_are_not_supported("MyIndexName", Span::new(285, 317)),
    );
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

    errors.assert_is(DatamodelError::new_model_validation_error(
        "The index definition refers to the unknown fields foo, bar.",
        "User",
        Span::new(48, 64),
    ));
}

#[test]
fn index_attributes_must_serialize_to_valid_dml() {
    let dml = r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@index([firstName,lastName], name: "customName")
        }
    "#;
    let schema = parse(dml);

    assert!(datamodel::parse_datamodel(&render_datamodel_to_string(&schema).unwrap()).is_ok());
}
