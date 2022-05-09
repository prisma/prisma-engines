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

    assert!(datamodel::parse_schema(dml).is_ok());
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

    assert!(datamodel::parse_schema(dml).is_ok());
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

    assert!(datamodel::parse_schema(dml).is_ok());
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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: The argument `references` must refer to a unique criteria in the related model `User`. But it is referencing the following fields that are not a unique criteria: name[0m
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

    assert!(datamodel::parse_schema(dml).is_ok());
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

    assert!(datamodel::parse_schema(dml).is_ok());
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

    let err = datamodel::parse_schema(schema).unwrap_err();

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
