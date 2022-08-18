use crate::{common::*, with_header, Provider};

#[test]
fn sqlite_disallows_compound_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(length: 10), b(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::Sqlite, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b String
        [1;94m14 | [0m  [1;91m@@unique([a(length: 10), b(length: 30)])[0m
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

    let dml = with_header(dml, Provider::Sqlite, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let dml = with_header(dml, Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id String @id @map("_id") @test.ObjectId
        [1;94m13 | [0m  val String [1;91m@unique(length: 30)[0m
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

    let dml = with_header(dml, Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b String
        [1;94m15 | [0m  [1;91m@@unique([a(length: 10), b(length: 30)])[0m
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

    let dml = with_header(dml, Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is only allowed with field types `String` or `Bytes`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": Hash type does not support sort option.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(sort: Desc)], type: Hash)[0m
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

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given index type is not supported with the current connector[0m
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
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": You must enable `fullTextIndex` preview feature to be able to define a @@fulltext index.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@fulltext([a, b])[0m
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

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given index type is not supported with the current connector[0m
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

    let schema = with_header(dml, Provider::Mysql, &["fullTextIndex"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": The length argument is not supported in a @@fulltext attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@fulltext([a(length: 30), b])[0m
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

    let schema = with_header(dml, Provider::Sqlite, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given index type is not supported with the current connector[0m
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

    let schema = with_header(dml, Provider::Mysql, &["fullTextIndex"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": The sort argument is not supported in a @@fulltext attribute in the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@fulltext([a(sort: Desc), b])[0m
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

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given index type is not supported with the current connector[0m
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
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": Defining fulltext indexes is not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@fulltext([a, b])[0m
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
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": Defining fulltext indexes is not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@fulltext([a, b])[0m
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
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": Defining fulltext indexes is not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@fulltext([a, b])[0m
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

    let schema = with_header(dml, Provider::Mongo, &["fullTextIndex"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": The current connector only allows one fulltext attribute per model[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m
        [1;94m18 | [0m  [1;91m@@fulltext([a, b])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": The current connector only allows one fulltext attribute per model[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m  @@fulltext([a, b])
        [1;94m19 | [0m  [1;91m@@fulltext([a, b, c, d])[0m
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

    let schema = with_header(dml, Provider::Mongo, &["fullTextIndex"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": All index fields must be listed adjacently in the fields argument.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m
        [1;94m18 | [0m  [1;91m@@fulltext([a, b(sort: Desc), c, d])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn index_without_fields_must_error() {
    let schema = r#"
        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["fullTextIndex"]
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
        [1;91merror[0m: [1mError parsing attribute "@@index": The list of fields in an index cannot be empty. Please specify at least one field.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m          @@fulltext(fields:[], map: "a")
        [1;94m18 | [0m          [1;91m@@index(fields: [ ], map: "b")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": The list of fields in an index cannot be empty. Please specify at least one field.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m          @@index(fields: [ ], map: "b")
        [1;94m19 | [0m          [1;91m@@unique(fields: [])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@fulltext": The list of fields in an index cannot be empty. Please specify at least one field.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m          [1;91m@@fulltext(fields:[], map: "a")[0m
        [1;94m   | [0m
    "#]];

    let error = parse_unwrap_err(schema);
    expected.assert_eq(&error);
}

#[test]
fn duplicate_indices_on_the_same_fields_are_not_allowed_on_mongodb() {
    let dml = indoc! {r#"
        model A {
          id   String @id @default(auto()) @map("_id") @test.ObjectId
          data Int

          @@index([data], map: "index_a")
          @@index([data], map: "index_b")
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": Index already exists in the model.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([data], map: "index_a")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@index": Index already exists in the model.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  @@index([data], map: "index_a")
        [1;94m16 | [0m  [1;91m@@index([data], map: "index_b")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn duplicate_uniques_on_the_same_fields_are_not_allowed_on_mongodb() {
    let dml = indoc! {r#"
        model A {
          id   String @id @default(auto()) @map("_id") @test.ObjectId
          data Int
          dota Int

          @@unique([data, dota], map: "index_a")
          @@unique([data, dota], map: "index_b")
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": Index already exists in the model.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@unique([data, dota], map: "index_a")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": Index already exists in the model.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  @@unique([data, dota], map: "index_a")
        [1;94m17 | [0m  [1;91m@@unique([data, dota], map: "index_b")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn duplicate_indices_on_the_same_fields_different_sort_same_name_are_not_allowed_on_mongodb() {
    let dml = indoc! {r#"
        model A {
          id   String @id @default(auto()) @map("_id") @test.ObjectId
          data Int

          @@index([data(sort: Asc)])
          @@index([data(sort: Desc)])
        }
    "#};

    let dml = with_header(dml, Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `A_data_idx` has to be unique in the following namespace: on model `A` for indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([data(sort: Asc)])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `A_data_idx` has to be unique in the following namespace: on model `A` for indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  @@index([data(sort: Asc)])
        [1;94m16 | [0m  [1;91m@@index([data(sort: Desc)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
