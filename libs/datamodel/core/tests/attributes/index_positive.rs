use crate::common::*;
use crate::{with_header, Provider};
use datamodel::{render_datamodel_to_string, IndexAlgorithm, IndexDefinition, IndexField, IndexType, SortOrder};

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
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
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
        fields: vec![IndexField::new("role")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
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
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
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
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
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
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
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
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("User_firstName_lastName_idx".to_string()),
        fields: vec![IndexField::new("firstName"), IndexField::new("lastName")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
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
    let schema = parse(&schema);
    let user_model = schema.assert_has_model("A");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_id_key".to_string()),
        fields: vec![IndexField {
            name: "id".to_string(),
            sort_order: None,
            length: Some(30),
        }],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
    });
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

#[test]
fn hash_index_works_on_postgres() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Hash)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    let schema = parse(&schema);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![IndexField::new("a")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Hash),
    });
}

#[test]
fn mysql_fulltext_index() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String
          b String

          @@fulltext([a, b])
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &["fullTextIndex"]);

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_b_idx".to_string()),
        fields: vec![IndexField::new("a"), IndexField::new("b")],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
    });
}

#[test]
fn mysql_fulltext_index_map() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String
          b String

          @@fulltext([a, b], map: "my_text_index")
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &["fullTextIndex"]);

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("my_text_index".to_string()),
        fields: vec![IndexField::new("a"), IndexField::new("b")],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
    });
}

#[test]
fn fulltext_index_mongodb() {
    let dml = indoc! {r#"
        model A {
          id String  @id @map("_id") @test.ObjectId
          a  String
          b  String

          @@fulltext([a, b])
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &["fullTextIndex", "mongodb"]);

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_b_idx".to_string()),
        fields: vec![IndexField::new("a"), IndexField::new("b")],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
    });
}

#[test]
fn fulltext_index_sort_mongodb() {
    let dml = indoc! {r#"
        model A {
          id String  @id @map("_id") @test.ObjectId
          a  String
          b  String

          @@fulltext([a, b(sort: Desc)])
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &["fullTextIndex", "extendedIndexes", "mongodb"]);

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_b_idx".to_string()),
        fields: vec![
            IndexField::new("a"),
            IndexField {
                name: "b".to_string(),
                sort_order: Some(SortOrder::Desc),
                length: None,
            },
        ],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
    });
}

#[test]
fn multiple_fulltext_indexes_allowed_per_model_in_mysql() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String
          b  String
          c  String
          d  String

          @@fulltext([a, b])
          @@fulltext([a, b, c, d])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["fullTextIndex"]);

    parse(&schema)
        .assert_has_model("A")
        .assert_has_index(IndexDefinition {
            name: None,
            db_name: Some("A_a_b_idx".to_string()),
            fields: vec![IndexField::new("a"), IndexField::new("b")],
            tpe: IndexType::Fulltext,
            algorithm: None,
            defined_on_field: false,
        })
        .assert_has_index(IndexDefinition {
            name: None,
            db_name: Some("A_a_b_c_d_idx".to_string()),
            fields: vec![
                IndexField::new("a"),
                IndexField::new("b"),
                IndexField::new("c"),
                IndexField::new("d"),
            ],
            tpe: IndexType::Fulltext,
            algorithm: None,
            defined_on_field: false,
        });
}
