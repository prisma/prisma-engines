use crate::common::*;

#[test]
fn multiple_indexes_with_same_name_on_different_models_are_supported_by_mysql() {
    for provider in ["mysql", "kingbase-mysql"] {
        let dml = formatdoc! {r#"
            datasource mysql {{
              provider = "{provider}"
            }}

            model User {{
              id         Int @id
              neighborId Int

              @@index([id], name: "MyIndexName")
            }}

            model Post {{
              id Int @id
              optionId Int

              @@index([id], name: "MyIndexName")
            }}
        "#};

        let schema = psl::parse_schema_without_extensions(&dml).unwrap();

        schema
            .assert_has_model("User")
            .assert_index_on_fields(&["id"])
            .assert_name("MyIndexName");

        schema
            .assert_has_model("Post")
            .assert_index_on_fields(&["id"])
            .assert_name("MyIndexName");

        let unique_dml = dml.replace("@@index", "@@unique");
        let schema = parse_schema(&unique_dml);
        schema
            .assert_has_model("User")
            .assert_unique_on_fields(&["id"])
            .assert_name("MyIndexName");
        schema
            .assert_has_model("Post")
            .assert_unique_on_fields(&["id"])
            .assert_name("MyIndexName");
    }
}

#[test]
fn foreign_keys_and_indexes_with_same_name_on_same_table_are_not_supported_on_mysql() {
    for provider in ["mysql", "kingbase-mysql"] {
        let dml = formatdoc! {r#"
            datasource mysql {{
              provider = "{provider}"
            }}

            model A {{
              id  Int @id
              bId Int
              b   B   @relation(fields: [bId], references: [id], map: "foo")

              @@index([bId], map: "foo")
            }}

            model B {{
              id Int @id
              as A[]
            }}
        "#};

        psl::parse_schema_without_extensions(&dml)
            .unwrap()
            .assert_has_model("A")
            .assert_index_on_fields(&["bId"])
            .assert_mapped_name("foo");
    }
}

#[test]
fn multiple_indexes_with_same_name_on_different_models_are_supported_by_mssql() {
    let dml = indoc! {r#"
        datasource sqlserver {
          provider = "sqlserver"
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

    let schema = psl::parse_schema_without_extensions(dml).unwrap();

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

    let schema = psl::parse_schema_without_extensions(dml).unwrap();

    schema
        .assert_has_model("User")
        .assert_index_on_fields(&["id"])
        .assert_name("MyName");

    schema
        .assert_has_model("Post")
        .assert_index_on_fields(&["id"])
        .assert_name("MyOtherName");
}

#[test]
fn constraint_names_can_be_reused_in_different_schemas() {
    let source = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
          schemas = ["one", "two"]
        }
        model A {
          id Int @id(map: "shared_pk")
          value Int
          @@index([value], map: "shared_index")
          @@unique([value], map: "shared_unique")
          @@schema("one")
        }
        model B {
          id Int @id(map: "shared_pk")
          value Int
          @@index([value], map: "shared_index")
          @@unique([value], map: "shared_unique")
          @@schema("two")
        }
    "#};
    let schema = parse_schema(source);
    for (name, namespace) in [("A", "one"), ("B", "two")] {
        let model = schema.assert_has_model(name);
        assert_eq!(model.schema_name(), Some(namespace));
        model.assert_id_on_fields(&["id"]).assert_mapped_name("shared_pk");
        model
            .assert_index_on_fields(&["value"])
            .assert_mapped_name("shared_index");
        model
            .assert_unique_on_fields(&["value"])
            .assert_mapped_name("shared_unique");
    }
}

#[test]
fn foreign_key_names_can_be_reused_on_different_tables() {
    let source = crate::with_header(
        indoc! {r#"
            model Parent {
              id Int @id
              first First[]
              second Second[]
            }
            model First {
              id Int @id
              parentId Int
              parent Parent @relation(fields: [parentId], references: [id], map: "shared_fk")
            }
            model Second {
              id Int @id
              parentId Int
              parent Parent @relation(fields: [parentId], references: [id], map: "shared_fk")
            }
        "#},
        crate::Provider::KingbaseOracle,
        &[],
    );
    let schema = parse_schema(&source);
    for name in ["First", "Second"] {
        assert_eq!(
            schema
                .assert_has_model(name)
                .assert_has_relation_field("parent")
                .mapped_name(),
            Some("shared_fk")
        );
    }
}
