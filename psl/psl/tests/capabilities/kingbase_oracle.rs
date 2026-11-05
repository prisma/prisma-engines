use crate::common::*;

#[test]
fn enum_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model Todo {
          id     Int    @id
          status Status
        }

        enum Status {
          DONE
          NOT_DONE
        }
    "#};

    assert_valid(dml)
}

#[test]
fn scalar_list_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model Todo {
          id     Int    @id
          val    String[]
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "val" in model "Todo" can't be a list. The current connector does not support lists of primitive types.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id     Int    @id
        [1;94m 7 | [0m  [1;91mval    String[][0m
        [1;94m 8 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn unique_index_names_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id         Int @id
          neighborId Int

          @@index([id], name: "metaId")
        }

        model Post {
          id Int @id
          optionId Int

          @@index([id], name: "metaId")
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `metaId` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m
        [1;94m 9 | [0m  @@index([id], [1;91mname: "metaId"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `metaId` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@index([id], [1;91mname: "metaId"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn json_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id   Int @id
          data Json
        }
    "#};

    assert_valid(dml)
}

#[test]
fn auto_increment_on_non_primary_column_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model Todo {
          id           Int    @id
          non_primary  Int    @default(autoincrement()) @unique
        }
    "#};

    assert_valid(dml)
}

#[test]
fn key_order_enforcement_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model  Todo {
          id1  Int
          id2  Int
          cats Cat[]

          @@id([id1, id2])
        }

        model Cat {
          id    Int @id
          todo1 Int
          todo2 Int

          rel Todo @relation(fields: [todo1, todo2], references: [id2, id1])
        }
    "#};

    assert_valid(dml)
}

#[test]
fn composite_types_are_not_supported() {
    let schema = r#"
        datasource db {
            provider = "kingbase-oracle"
        }

        type Address {
            street String
        }
    "#;

    let err = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Composite types are not supported on Kingbase Oracle.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m        [1;91mtype Address {[0m
        [1;94m 7 | [0m            street String
        [1;94m 8 | [0m        }
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&err);
}

#[test]
fn fulltext_indexes_require_connector_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }
        model Article {
          id Int @id
          body String
          @@fulltext([body])
        }
    "#};
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": Defining fulltext indexes is not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  body String
        [1;94m 7 | [0m  [1;91m@@fulltext([body])[0m
        [1;94m   | [0m
    "#]];
    expect_error(dml, &expected);
}
