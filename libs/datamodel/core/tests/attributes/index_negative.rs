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
        [1;91merror[0m: [1mError parsing attribute "@unique": You must enable `extendedIndexes` preview feature to use sort or length parameters.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m         id         Int    @id
        [1;94m14 | [0m         firstName  String @[1;91munique(sort:Desc, length: 5)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": You must enable `extendedIndexes` preview feature to use sort or length parameters.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m         firstName  String @unique(sort:Desc, length: 5)
        [1;94m15 | [0m         middleName String @[1;91munique(sort:Desc)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": You must enable `extendedIndexes` preview feature to use sort or length parameters.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m         middleName String @unique(sort:Desc)
        [1;94m16 | [0m         lastName   String @[1;91munique(length: 5)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@index": You must enable `extendedIndexes` preview feature to use sort or length parameters.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m         
        [1;94m19 | [0m         @@[1;91mindex([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": You must enable `extendedIndexes` preview feature to use sort or length parameters.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m         @@index([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
        [1;94m20 | [0m         @@[1;91munique([firstName(sort: Desc), middleName(length: 6), lastName(sort: Desc, length: 6), generation])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn index_does_not_accept_missing_length_with_extended_indexes() {
    let dml = with_header(
        r#"
     model User {
         id         Int    @id
         firstName  String @unique @test.Text
         
         @@index([firstName])
     }
     "#,
        Provider::Mysql,
        &["extendedIndexes"],
    );

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type Text cannot be unique in MySQL. If you are using the `extendedIndexes` preview feature you can add a `length` argument to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m
        [1;94m12 | [0m     [1;91mmodel User {[0m
        [1;94m13 | [0m         id         Int    @id
        [1;94m14 | [0m         firstName  String @unique @test.Text
        [1;94m15 | [0m         
        [1;94m16 | [0m         @@index([firstName])
        [1;94m17 | [0m     }
        [1;94m   | [0m
        [1;91merror[0m: [1mYou cannot define an index on fields with Native type Text of MySQL. If you are using the `extendedIndexes` preview feature you can add a `length` argument to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m
        [1;94m12 | [0m     [1;91mmodel User {[0m
        [1;94m13 | [0m         id         Int    @id
        [1;94m14 | [0m         firstName  String @unique @test.Text
        [1;94m15 | [0m         
        [1;94m16 | [0m         @@index([firstName])
        [1;94m17 | [0m     }
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

#[test]
fn hash_index_doesnt_allow_sorting() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(sort: Desc)], type: Hash)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": Hash type does not support sort option.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mindex([a(sort: Desc)], type: Hash)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn hash_index_doesnt_work_on_sqlserver() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Hash)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The given type argument is not supported with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([a], [1;91mtype: Hash[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fulltext_index_no_preview_feature() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String
          b String

          @@fulltext([a, b])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@fulltext": You must enable `fullTextIndex` preview feature to be able to define a @@fulltext index.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mfulltext([a, b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn hash_index_doesnt_work_on_mysql() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Hash)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The given type argument is not supported with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([a], [1;91mtype: Hash[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fulltext_index_length_attribute() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String
          b String

          @@fulltext([a(length: 30), b])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["fullTextIndex", "extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The length argument is not supported in a @@fulltext attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mfulltext([a(length: 30), b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn hash_index_doesnt_work_on_sqlite() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Hash)
        }
    "#};

    let schema = with_header(dml, Provider::Sqlite, &["extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The given type argument is not supported with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([a], [1;91mtype: Hash[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fulltext_index_sort_attribute() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String
          b String

          @@fulltext([a(sort: Desc), b])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &["fullTextIndex", "extendedIndexes"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The sort argument is not supported in a @@fulltext attribute in the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mfulltext([a(sort: Desc), b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn hash_index_doesnt_work_on_mongo() {
    let dml = indoc! {r#"
        model A {
          id Int @id @map("_id")
          a  Int

          @@index([a], type: Hash)
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &["extendedIndexes", "mongoDb"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The given type argument is not supported with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([a], [1;91mtype: Hash[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fulltext_index_postgres() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String
          b  String

          @@fulltext([a, b])
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &["fullTextIndex"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@fulltext": Defining fulltext indexes is not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mfulltext([a, b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fulltext_index_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String
          b  String

          @@fulltext([a, b])
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &["fullTextIndex"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@fulltext": Defining fulltext indexes is not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mfulltext([a, b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fulltext_index_sqlite() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String
          b  String

          @@fulltext([a, b])
        }
    "#};

    let schema = with_header(dml, Provider::Sqlite, &["fullTextIndex"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@fulltext": Defining fulltext indexes is not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mfulltext([a, b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn only_one_fulltext_index_allowed_per_model_in_mongo() {
    let dml = indoc! {r#"
        model A {
          id String  @id @map("_id") @test.ObjectId
          a  String
          b  String
          c  String
          d  String

          @@fulltext([a, b])
          @@fulltext([a, b, c, d])
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &["fullTextIndex", "mongodb"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@fulltext": The current connector only allows one fulltext attribute per model[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m
        [1;94m18 | [0m  @@[1;91mfulltext([a, b])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@fulltext": The current connector only allows one fulltext attribute per model[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m  @@fulltext([a, b])
        [1;94m19 | [0m  @@[1;91mfulltext([a, b, c, d])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fulltext_index_fields_must_follow_each_other_in_mongo() {
    let dml = indoc! {r#"
        model A {
          id String  @id @map("_id") @test.ObjectId
          a  String
          b  String
          c  String
          d  String

          @@fulltext([a, b(sort: Desc), c, d])
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &["fullTextIndex", "extendedIndexes", "mongodb"]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@fulltext": All index fields must be listed adjacently in the fields argument.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m
        [1;94m18 | [0m  @@[1;91mfulltext([a, b(sort: Desc), c, d])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn index_without_fields_must_error() {
    let schema = r#"
        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["fullTextIndex", "extendedIndexes"]
        }

        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model Fulltext {
          id      Int    @id
          title   String @db.VarChar(255)
          content String @db.Text

          @@fulltext(fields:[], map: "a")
          @@index(fields: [ ], map: "b")
          @@unique(fields: [])
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The list of fields in an index cannot be empty. Please specify at least one field.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m          @@fulltext(fields:[], map: "a")
        [1;94m18 | [0m          @@[1;91mindex(fields: [ ], map: "b")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": The list of fields in an index cannot be empty. Please specify at least one field.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m          @@index(fields: [ ], map: "b")
        [1;94m19 | [0m          @@[1;91munique(fields: [])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@fulltext": The list of fields in an index cannot be empty. Please specify at least one field.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m          @@[1;91mfulltext(fields:[], map: "a")[0m
        [1;94m   | [0m
    "#]];

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();
    expected.assert_eq(&error);
}
