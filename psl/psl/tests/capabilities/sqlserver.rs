use crate::common::*;
use indoc::indoc;

#[test]
fn enum_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "file:test.db"
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

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: You defined the enum `Status`. But the current connector does not support enums.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91menum Status {[0m
        [1;94m12 | [0m  DONE
        [1;94m13 | [0m  NOT_DONE
        [1;94m14 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn scalar_list_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "file:test.db"
        }

        model Todo {
          id     Int    @id
          val    String[]
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "val" in model "Todo" can't be a list. The current connector does not support lists of primitive types.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int    @id
        [1;94m 8 | [0m  [1;91mval    String[][0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn unique_index_names_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
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

    assert_valid(dml);
}

#[test]
fn json_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model User {
          id   Int @id
          data Json
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating field `data` in model `User`: Field `data` in model `User` can't be of type Json. The current connector does not support the Json type.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id   Int @id
        [1;94m 8 | [0m  [1;91mdata Json[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn non_unique_relation_criteria_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
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
          provider = "sqlserver"
          url = "sqlserver://"
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
          provider = "sqlserver"
          url = "sqlserver://"
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

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: The argument `references` must refer to a unique criterion in the related model `Todo` using the same order of fields. Please check the ordering in the following fields: `id2, id1`.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91mrel Todo @relation(fields: [todo1, todo2], references: [id2, id1])[0m
        [1;94m20 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn postgres_does_not_support_composite_types() {
    let schema = r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        type Address {
            street String
        }
    "#;

    let err = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Composite types are not supported on SQL Server.[0m
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
