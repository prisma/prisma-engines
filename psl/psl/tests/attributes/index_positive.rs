use crate::{common::*, with_header, Provider};
use psl::render_datamodel_to_string;

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
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
        fields: vec![IndexField::new_in_model("role")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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

    assert_valid(&render_datamodel_to_string(&schema, None))
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
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("User_firstName_lastName_idx".to_string()),
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn mysql_allows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @unique(length: 30) @test.VarChar(255)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    let schema = parse(&schema);
    let user_model = schema.assert_has_model("A");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_id_key".to_string()),
        fields: vec![IndexField {
            path: vec![("id".to_string(), None)],
            sort_order: None,
            length: Some(30),
            operator_class: None,
        }],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
        clustered: None,
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
}

#[test]
fn mysql_allows_index_length_prefix_on_unsupported_field() {
    let dml = indoc! {r#"
        model A {
          id Int                     @id
          a  Unsupported("geometry")

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
}

#[test]
fn mysql_allows_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
}

#[test]
fn sqlite_allows_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::Sqlite, &[]);
    assert_valid(&schema);
}

#[test]
fn sqlserver_allows_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    assert_valid(&schema);
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
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

    let schema = with_header(dml, Provider::Sqlite, &[]);
    assert_valid(&schema);
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

    let schema = with_header(dml, Provider::SqlServer, &[]);
    assert_valid(&schema);
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
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

    let schema = with_header(dml, Provider::SqlServer, &[]);
    assert_valid(&schema);
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
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
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
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
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

    let dml = with_header(dml, Provider::Mongo, &["fullTextIndex"]);

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_b_idx".to_string()),
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });
}

#[test]
fn duplicate_index_different_sort_order_mongodb() {
    let dml = indoc! {r#"
        model A {
          id String @id @default(auto()) @map("_id") @test.ObjectId
          a  Int

          @@index([a(sort: Asc)], map: "aaa")
          @@index([a(sort: Desc)], map: "bbb")
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &[]);

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("aaa".to_string()),
        fields: vec![IndexField {
            path: vec![("a".to_string(), None)],
            sort_order: Some(SortOrder::Asc),
            length: None,
            operator_class: None,
        }],
        tpe: IndexType::Normal,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("bbb".to_string()),
        fields: vec![IndexField {
            path: vec![("a".to_string(), None)],
            sort_order: Some(SortOrder::Desc),
            length: None,
            operator_class: None,
        }],
        tpe: IndexType::Normal,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
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

    let dml = with_header(dml, Provider::Mongo, &["fullTextIndex"]);

    parse(&dml).assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_b_idx".to_string()),
        fields: vec![
            IndexField::new_in_model("a"),
            IndexField {
                path: vec![("b".to_string(), None)],
                sort_order: Some(SortOrder::Desc),
                length: None,
                operator_class: None,
            },
        ],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
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
            fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
            tpe: IndexType::Fulltext,
            algorithm: None,
            defined_on_field: false,
            clustered: None,
        })
        .assert_has_index(IndexDefinition {
            name: None,
            db_name: Some("A_a_b_c_d_idx".to_string()),
            fields: vec![
                IndexField::new_in_model("a"),
                IndexField::new_in_model("b"),
                IndexField::new_in_model("c"),
                IndexField::new_in_model("d"),
            ],
            tpe: IndexType::Fulltext,
            algorithm: None,
            defined_on_field: false,
            clustered: None,
        });
}
