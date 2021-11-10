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
fn mysql_allows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @unique(length: 30) @test.VarChar(255)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn mysql_allows_compound_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(length: 10), b(length: 30)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn mysql_allows_index_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn mysql_allows_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn postgres_allows_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn sqlite_allows_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::Sqlite, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn sqlserver_allows_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn mysql_allows_compound_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(sort: Desc), b(sort: Asc)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn postgres_allows_compound_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(sort: Desc), b(sort: Asc)])
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn sqlite_allows_compound_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(sort: Desc), b(sort: Asc)])
        }
    "#};

    let schema = with_header(dml, Provider::Sqlite, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn sqlserver_allows_compound_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(sort: Desc), b(sort: Asc)])
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn mysql_allows_index_sort_order() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(sort: Desc)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn postrgres_allows_index_sort_order() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(sort: Desc)])
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}

#[test]
fn sqlserver_allows_index_sort_order() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(sort: Desc)])
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &["extendedIndexes"]);
    assert!(datamodel::parse_schema(&schema).is_ok());
}
