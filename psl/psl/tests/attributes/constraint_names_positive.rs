use crate::common::*;

#[test]
fn multiple_indexes_with_same_name_on_different_models_are_supported_by_mysql() {
    let dml = indoc! {r#"
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
    "#};

    let schema = psl::parse_schema(dml).unwrap();

    schema
        .assert_has_model("User")
        .assert_index_on_fields(&["id"])
        .assert_name("MyIndexName");

    schema
        .assert_has_model("Post")
        .assert_index_on_fields(&["id"])
        .assert_name("MyIndexName");
}

#[test]
fn foreign_keys_and_indexes_with_same_name_on_same_table_are_not_supported_on_mysql() {
    let dml = indoc! {r#"
        datasource mysql {
          provider = "mysql"
          url = "mysql://asdlj"
        }

        model A {
          id  Int @id
          bId Int
          b   B   @relation(fields: [bId], references: [id], map: "foo")
          
          @@index([bId], map: "foo")
        }
        
        model B {
          id Int @id
          as A[]
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["bId"])
        .assert_mapped_name("foo");
}

#[test]
fn multiple_indexes_with_same_name_on_different_models_are_supported_by_mssql() {
    let dml = indoc! {r#"
        datasource sqlserver {
          provider = "sqlserver"
          url = "sqlserver://asdlj"
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
    "#};

    let schema = psl::parse_schema(dml).unwrap();

    schema
        .assert_has_model("User")
        .assert_index_on_fields(&["id"])
        .assert_name("MyIndexName");

    schema
        .assert_has_model("Post")
        .assert_index_on_fields(&["id"])
        .assert_name("MyIndexName");
}

#[test]
fn multiple_constraints_with_same_name_in_different_namespaces_are_supported_by_mssql() {
    let dml = indoc! {r#"
        datasource sqlserver {
          provider = "sqlserver"
          url = "sqlserver://asdlj"
        }

        model User {
          id         Int    @id
          neighborId Int    @default(5, map: "MyName")
          posts      Post[]

          @@index([id], name: "MyName")
        }

        model Post {
          id     Int  @id
          userId Int
          User   User @relation(fields: [userId], references: [id], map: "MyOtherName")

          @@index([id], name: "MyOtherName")
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();

    schema
        .assert_has_model("User")
        .assert_index_on_fields(&["id"])
        .assert_name("MyName");

    schema
        .assert_has_model("Post")
        .assert_index_on_fields(&["id"])
        .assert_name("MyOtherName");
}
