use crate::{Provider, common::*, with_header};

#[test]
fn clustered_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int

          @@index([a], clustered: true)
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_clustered(true);
}

#[test]
fn clustered_unique_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int @unique(clustered: true)
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_unique_on_fields(&["a"])
        .assert_clustered(true);
}

#[test]
fn clustered_compound_unique_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int
          b  Int

          @@unique([a, b], clustered: true)
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_unique_on_fields(&["a", "b"])
        .assert_clustered(true);
}

#[test]
fn non_clustered_id_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_id_on_fields(&["id"])
        .assert_clustered(false);
}

#[test]
fn non_clustered_compound_id_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          left  Int
          right Int

          @@id([left, right], clustered: false)
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_id_on_fields(&["left", "right"])
        .assert_clustered(false);
}

#[test]
fn clustered_keys_and_indexes_require_connector_support() {
    for provider in ["kingbase-mysql", "kingbase-oracle"] {
        for key in [
            "@id(clustered: true)",
            "@unique(clustered: true)",
            "@@index([value], clustered: true)",
        ] {
            let id = if key.starts_with("@id") { key } else { "@id" };
            let value = if key.starts_with("@unique") {
                " @unique(clustered: true)"
            } else {
                ""
            };
            let model = if key.starts_with("@@") { key } else { "" };
            let source = formatdoc! {r#"
                datasource db {{
                  provider = "{provider}"
                }}
                model Item {{
                  id Int {id}
                  value Int{value}
                  {model}
                }}
            "#};
            let expect = match key {
                "@id(clustered: true)" => expect![[r#"
                    [1;91merror[0m: [1mError parsing attribute "@id": Defining clustering is not supported in the current connector.[0m
                      [1;94m-->[0m  [4mschema.prisma:5[0m
                    [1;94m   | [0m
                    [1;94m 4 | [0mmodel Item {
                    [1;94m 5 | [0m  id Int [1;91m@id(clustered: true)[0m
                    [1;94m   | [0m
                "#]],
                "@unique(clustered: true)" => expect![[r#"
                    [1;91merror[0m: [1mError parsing attribute "@unique": Defining clustering is not supported in the current connector.[0m
                      [1;94m-->[0m  [4mschema.prisma:6[0m
                    [1;94m   | [0m
                    [1;94m 5 | [0m  id Int @id
                    [1;94m 6 | [0m  value Int [1;91m@unique(clustered: true)[0m
                    [1;94m   | [0m
                "#]],
                "@@index([value], clustered: true)" => expect![[r#"
                    [1;91merror[0m: [1mError parsing attribute "@@index": Defining clustering is not supported in the current connector.[0m
                      [1;94m-->[0m  [4mschema.prisma:7[0m
                    [1;94m   | [0m
                    [1;94m 6 | [0m  value Int
                    [1;94m 7 | [0m  [1;91m@@index([value], clustered: true)[0m
                    [1;94m   | [0m
                "#]],
                _ => unreachable!(),
            };
            expect.assert_eq(&parse_unwrap_err(&source));
        }
    }
}
