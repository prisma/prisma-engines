use crate::{common::*, with_header, Provider};

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

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["firstName", "lastName"]);
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

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["role"]);
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

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["firstName", "lastName"])
        .assert_name("MyIndexName");
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

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["firstName", "lastName"])
        .assert_mapped_name("MyIndexName");
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

    psl::parse_schema(dml).unwrap();
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

    psl::parse_schema(dml).unwrap();
}

#[test]
fn mysql_allows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @unique(length: 30) @test.VarChar(255)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Mysql, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_unique_on_fields(&["id"])
        .assert_field("id")
        .assert_length(30);
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

    psl::parse_schema(with_header(dml, Provider::Mysql, &["fullTextIndex"]))
        .unwrap()
        .assert_has_model("A")
        .assert_fulltext_on_fields(&["a", "b"]);
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

    psl::parse_schema(with_header(dml, Provider::Mysql, &["fullTextIndex"]))
        .unwrap()
        .assert_has_model("A")
        .assert_fulltext_on_fields(&["a", "b"])
        .assert_mapped_name("my_text_index");
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

    psl::parse_schema(with_header(dml, Provider::Mongo, &["fullTextIndex"]))
        .unwrap()
        .assert_has_model("A")
        .assert_fulltext_on_fields(&["a", "b"]);
}

#[test]
fn duplicate_index_different_sort_order_mongodb() {
    let dml = indoc! {r#"
        model A {
          id String @id @default(auto()) @map("_id") @test.ObjectId
          a  Int

          @@index([a(sort: Desc)], map: "bbb")
          @@index([a(sort: Asc)], map: "aaa")
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Mongo, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_mapped_name("bbb")
        .assert_field("a")
        .assert_descending();
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

    psl::parse_schema(with_header(dml, Provider::Mongo, &["fullTextIndex"]))
        .unwrap()
        .assert_has_model("A")
        .assert_fulltext_on_fields(&["a", "b"])
        .assert_field("b")
        .assert_descending();
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

    let schema = psl::parse_schema(with_header(dml, Provider::Mysql, &["fullTextIndex"])).unwrap();
    let a = schema.assert_has_model("A");

    a.assert_fulltext_on_fields(&["a", "b"]);
    a.assert_fulltext_on_fields(&["a", "b", "c", "d"]);
}
