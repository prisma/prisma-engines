use datamodel::{render_datamodel_to_string, IndexDefinition, IndexType};

use crate::attributes::{with_header, Provider};
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
        db_name: Some("User_firstName_lastName_idx".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
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
        db_name: Some("User_role_idx".to_string()),
        fields: vec!["role".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

// Illustrates the @@index compatibility hack.
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
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn the_map_argument_must_work() {
    let dml = r#"
        datasource test {
            provider = "postgres"
            url = "postgresql://"
        }

        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@index([firstName,lastName], map: "MyIndexName")
        }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
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
        db_name: Some("User_firstName_lastName_idx".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
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

    assert!(datamodel::parse_datamodel(&render_datamodel_to_string(&schema, None)).is_ok());
}

#[test]
fn index_accepts_three_different_notations() {
    let dml = with_header(
        r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        // compatibility
        @@index([firstName,lastName], map: "OtherIndexName")
        //explicit
        @@index([firstName,lastName], name: "MyIndexName")
        //implicit
        @@index([firstName,lastName])
    }
    "#,
        Provider::Postgres,
        &[],
    );

    let schema = parse(&dml);
    let user_model = schema.assert_has_model("User");

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("OtherIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("User_firstName_lastName_idx".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn index_accepts_sort_order() {
    let dml = with_header(
        r#"
     model User {
         id         Int    @id
         firstName  String @unique(sort:Desc, length: 5)
         middleName String @unique(sort:Desc)
         lastName   String @unique(length: 5)
         generation Int    @unique
         
         @@index([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
         @@unique([firstName(sort: Desc), middleName(length: 6), lastName(sort: Desc, length: 6), generation])
     }
     "#,
        Provider::Postgres,
        &["extendedIndexes"],
    );

    let schema = parse(&dml);
    schema.assert_has_model("User");

    // user_model.assert_has_index(IndexDefinition {
    //     name: None,
    //     db_name: Some("User_firstName_key".to_string()),
    //     fields: vec![("firstName".to_string(), Some(SortOrder::Desc), Some(5))],
    //     tpe: IndexType::Unique,
    //     defined_on_field: true,
    // });
    //
    // user_model.assert_has_index(IndexDefinition {
    //     name: None,
    //     db_name: Some("User_middleName_key".to_string()),
    //     fields: vec![("middleName".to_string(), Some(SortOrder::Desc), None)],
    //     tpe: IndexType::Unique,
    //     defined_on_field: true,
    // });
    //
    // user_model.assert_has_index(IndexDefinition {
    //     name: None,
    //     db_name: Some("User_lastName_key".to_string()),
    //     fields: vec![("lastName".to_string(), Some(SortOrder::Asc), Some(5))],
    //     tpe: IndexType::Unique,
    //     defined_on_field: true,
    // });
    //
    // user_model.assert_has_index(IndexDefinition {
    //     name: None,
    //     db_name: Some("User_generation_key".to_string()),
    //     fields: vec![("generation".to_string(), Some(SortOrder::Asc), None)],
    //     tpe: IndexType::Unique,
    //     defined_on_field: true,
    // });
    //
    // user_model.assert_has_index(IndexDefinition {
    //     name: None,
    //     db_name: Some("User_firstName_middleName_lastName_generation_idx".to_string()),
    //     fields: vec![
    //         ("firstName".to_string(), Some(SortOrder::Desc), None),
    //         ("middleName".to_string(), Some(SortOrder::Asc), Some(5)),
    //         ("lastName".to_string(), Some(SortOrder::Desc), Some(5)),
    //         ("generation".to_string(), Some(SortOrder::Asc), None),
    //     ],
    //     tpe: IndexType::Normal,
    //     defined_on_field: false,
    // });
    //
    // user_model.assert_has_index(IndexDefinition {
    //     name: None,
    //     db_name: Some("User_firstName_middleName_lastName_generation_key".to_string()),
    //     fields: vec![
    //         ("firstName".to_string(), Some(SortOrder::Desc), None),
    //         ("middleName".to_string(), Some(SortOrder::Asc), Some(6)),
    //         ("lastName".to_string(), Some(SortOrder::Desc), Some(6)),
    //         ("generation".to_string(), Some(SortOrder::Asc), None),
    //     ],
    //     tpe: IndexType::Unique,
    //     defined_on_field: false,
    // });
}
