use crate::common::*;
use indoc::indoc;

#[test]
fn enum_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url = "postgres://"
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
          provider = "cockroachdb"
          url = "postgres://"
        }

        model Todo {
          id     Int    @id
          val    String[]
        }
    "#};

    assert_valid(dml)
}

#[test]
fn json_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url = "postgres://"
        }

        model User {
          id   Int @id
          data Json
        }
    "#};

    assert_valid(dml)
}

#[test]
fn non_unique_relation_criteria_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
        }

        model Todo {
          id           Int    @id
          assigneeName String
          assignee     User   @relation(fields: [assigneeName], references: [name])
        }

        model User {
          id   Int    @id
          name String
          todos Todo[]
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The argument `references` must refer to a unique criterion in the related model. Consider adding an `@unique` attribute to the field `name` in the model `User`.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  assigneeName String
        [1;94m 9 | [0m  [1;91massignee     User   @relation(fields: [assigneeName], references: [name])[0m
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn auto_increment_on_non_primary_column_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url = "postgres://"
        }

        model Todo {
          id           Int    @id
          non_primary  BigInt    @default(autoincrement()) @unique
        }
    "#};

    assert_valid(dml)
}

#[test]
fn key_order_enforcement_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url = "postgres://"
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
fn does_not_support_composite_types() {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = "postgres://"
        }

        type Address {
            street String
        }
    "#;

    let err = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Composite types are not supported on CockroachDB.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m
        [1;94m 7 | [0m        [1;91mtype Address {[0m
        [1;94m 8 | [0m            street String
        [1;94m 9 | [0m        }
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&err);
}
