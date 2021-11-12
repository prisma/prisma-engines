use crate::common::*;
use crate::{with_header, Provider};

#[test]
fn indexes_on_relation_fields_must_error() {
    let dml = indoc! {r#"
        model User {
          id               Int @id
          identificationId Int

          identification   Identification @relation(fields: [identificationId], references:[id])

          @@index([identification])
        }

        model Identification {
          id Int @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The index definition refers to the relation fields identification. Index definitions must reference only scalar fields. Did you mean `@@index([identificationId])`?[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m
        [1;94m 7 | [0m  @@[1;91mindex([identification])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_when_unknown_fields_are_used() {
    let dml = indoc! {r#"
        model User {
          id Int @id

          @@index([foo,bar])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The index definition refers to the unknown fields foo, bar.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m
        [1;94m 4 | [0m  @@[1;91mindex([foo,bar])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn stringified_field_names_in_index_return_nice_error() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@index(["firstName", "lastName"])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a constant literal value, but received string value `"firstName"`.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@index([[1;91m"firstName"[0m, "lastName"])
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn index_does_not_accept_sort_or_length_without_extended_indexes() {
    let dml = with_header(
        r#"
     model User {
         id         Int    @id
         firstName  String @unique(sort:Desc, length: 5)
         middleName String @unique(sort:Desc)
         lastName   String @unique(length: 5)
         generation Int    @unique
         
         @@index([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
         @@unique([firstName(sort: Desc), middleName(length: 6), lastName(sort: Desc, length: 6), generation])
     }
     "#,
        Provider::Mysql,
        &[],
    );

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The sort and length arguments are not yet available.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m         id         Int    @id
        [1;94m14 | [0m         firstName  String @[1;91munique(sort:Desc, length: 5)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": The sort and length arguments are not yet available.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m         firstName  String @unique(sort:Desc, length: 5)
        [1;94m15 | [0m         middleName String @[1;91munique(sort:Desc)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": The sort and length arguments are not yet available.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m         middleName String @unique(sort:Desc)
        [1;94m16 | [0m         lastName   String @[1;91munique(length: 5)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@index": The sort and length arguments are not yet available.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m         
        [1;94m19 | [0m         @@[1;91mindex([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": The sort and length arguments are not yet available.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m         @@index([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
        [1;94m20 | [0m         @@[1;91munique([firstName(sort: Desc), middleName(length: 6), lastName(sort: Desc, length: 6), generation])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn postgres_disallows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @unique(length: 30) @test.VarChar(255)
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id String @[1;91munique(length: 30)[0m @test.VarChar(255)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn postgres_disallows_compound_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(length: 10), b(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b String
        [1;94m14 | [0m  @@[1;91munique([a(length: 10), b(length: 30)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn postgres_disallows_index_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(length: 10)])
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn sqlserver_disallows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @unique(length: 30) @test.VarChar(255)
        }
    "#};

    let dml = with_header(dml, Provider::SqlServer, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id String @[1;91munique(length: 30)[0m @test.VarChar(255)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn sqlserver_disallows_compound_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(length: 10), b(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::SqlServer, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b String
        [1;94m14 | [0m  @@[1;91munique([a(length: 10), b(length: 30)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn sqlserver_disallows_index_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(length: 10)])
        }
    "#};

    let dml = with_header(dml, Provider::SqlServer, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn sqlite_disallows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @unique(length: 30)
        }
    "#};

    let dml = with_header(dml, Provider::Sqlite, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id String @[1;91munique(length: 30)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn sqlite_disallows_compound_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(length: 10), b(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::Sqlite, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b String
        [1;94m14 | [0m  @@[1;91munique([a(length: 10), b(length: 30)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn sqlite_disallows_index_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(length: 10)])
        }
    "#};

    let dml = with_header(dml, Provider::Sqlite, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mongodb_disallows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @id @map("_id") @test.ObjectId
          val String @unique(length: 30)
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &["extendedIndexes", "mongoDb"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id String @id @map("_id") @test.ObjectId
        [1;94m13 | [0m  val String @[1;91munique(length: 30)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mongodb_disallows_compound_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @id @map("_id") @test.ObjectId
          a String
          b String
          @@unique([a(length: 10), b(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &["extendedIndexes", "mongoDb"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b String
        [1;94m15 | [0m  @@[1;91munique([a(length: 10), b(length: 30)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mongodb_disallows_index_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @id @map("_id") @test.ObjectId
          a String

          @@index([a(length: 10)])
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &["extendedIndexes", "mongoDb"]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn length_argument_does_not_work_with_decimal() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a Decimal

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn length_argument_does_not_work_with_json() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a Json

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn length_argument_does_not_work_with_datetime() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a DateTime

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn length_argument_does_not_work_with_boolean() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a Boolean

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn length_argument_does_not_work_with_float() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a Float

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn length_argument_does_not_work_with_bigint() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a BigInt

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn length_argument_does_not_work_with_int() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a Int

          @@index([a(length: 10)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
