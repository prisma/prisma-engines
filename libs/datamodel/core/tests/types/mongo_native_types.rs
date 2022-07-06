use expect_test::expect;
use indoc::indoc;
use native_types::MongoDbType;

use crate::{common::*, with_header, Provider};

#[test]
fn valid_json_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int  @id            @map("_id")
          a  Json @test.Json
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let (_, datamodel) = datamodel::parse_schema(&schema).unwrap();

    let model = datamodel.assert_has_model("A");

    let nt = model.assert_has_scalar_field("a").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::Json, mongo_type);
}

#[test]
fn valid_object_id_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int    @id            @map("_id")
          a  String @test.ObjectId
          b  Bytes  @test.ObjectId
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let (_, datamodel) = datamodel::parse_schema(&schema).unwrap();

    let model = datamodel.assert_has_model("A");

    let nt = model.assert_has_scalar_field("a").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::ObjectId, mongo_type);

    let nt = model.assert_has_scalar_field("b").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::ObjectId, mongo_type);
}

#[test]
fn valid_long_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int    @id            @map("_id")
          a  Int    @test.Long
          b  BigInt @test.Long
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let (_, datamodel) = datamodel::parse_schema(&schema).unwrap();

    let model = datamodel.assert_has_model("A");

    let nt = model.assert_has_scalar_field("a").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::Long, mongo_type);

    let nt = model.assert_has_scalar_field("b").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::Long, mongo_type);
}

#[test]
fn invalid_string_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  Int      @test.String
          b  Float    @test.String
          c  Bytes    @test.String
          d  Boolean  @test.String
          e  DateTime @test.String
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Int, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  Int      [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Float, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  Int      @test.String
        [1;94m14 | [0m  b  Float    [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Bytes, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.String
        [1;94m15 | [0m  c  Bytes    [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Boolean, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  c  Bytes    @test.String
        [1;94m16 | [0m  d  Boolean  [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type DateTime, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  d  Boolean  @test.String
        [1;94m17 | [0m  e  DateTime [1;91m@test.String[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_string_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  Int      @test.String
          b  Float    @test.String
          c  Bytes    @test.String
          d  Boolean  @test.String
          e  DateTime @test.String
        }

        model A {
          id Int @id @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Int, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  Int      [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Float, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  Int      @test.String
        [1;94m13 | [0m  b  Float    [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Bytes, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.String
        [1;94m14 | [0m  c  Bytes    [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type Boolean, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  c  Bytes    @test.String
        [1;94m15 | [0m  d  Boolean  [1;91m@test.String[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type String is not compatible with declared field type DateTime, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Boolean  @test.String
        [1;94m16 | [0m  e  DateTime [1;91m@test.String[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_double_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  Int      @test.Double
          b  BigInt   @test.Double
          d  Boolean  @test.Double
          e  String   @test.Double
          f  DateTime @test.Double
          g  Bytes    @test.Double
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type Int, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  Int      [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type BigInt, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  Int      @test.Double
        [1;94m14 | [0m  b  BigInt   [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type Boolean, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  BigInt   @test.Double
        [1;94m15 | [0m  d  Boolean  [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type String, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Boolean  @test.Double
        [1;94m16 | [0m  e  String   [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type DateTime, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  e  String   @test.Double
        [1;94m17 | [0m  f  DateTime [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type Bytes, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  f  DateTime @test.Double
        [1;94m18 | [0m  g  Bytes    [1;91m@test.Double[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_double_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  Int      @test.Double
          b  BigInt   @test.Double
          d  Boolean  @test.Double
          e  String   @test.Double
          f  DateTime @test.Double
          g  Bytes    @test.Double
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type Int, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  Int      [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type BigInt, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  Int      @test.Double
        [1;94m13 | [0m  b  BigInt   [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type Boolean, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  BigInt   @test.Double
        [1;94m14 | [0m  d  Boolean  [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type String, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Boolean  @test.Double
        [1;94m15 | [0m  e  String   [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type DateTime, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  e  String   @test.Double
        [1;94m16 | [0m  f  DateTime [1;91m@test.Double[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Double is not compatible with declared field type Bytes, expected field type Float.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  DateTime @test.Double
        [1;94m17 | [0m  g  Bytes    [1;91m@test.Double[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_long_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          b  Float    @test.Long
          d  Boolean  @test.Long
          e  String   @test.Long
          f  DateTime @test.Long
          g  Bytes    @test.Long
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type Float, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  b  Float    [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type Boolean, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.Long
        [1;94m14 | [0m  d  Boolean  [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type String, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Boolean  @test.Long
        [1;94m15 | [0m  e  String   [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type DateTime, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  e  String   @test.Long
        [1;94m16 | [0m  f  DateTime [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type Bytes, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  DateTime @test.Long
        [1;94m17 | [0m  g  Bytes    [1;91m@test.Long[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_long_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          b  Float    @test.Long
          d  Boolean  @test.Long
          e  String   @test.Long
          f  DateTime @test.Long
          g  Bytes    @test.Long
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type Float, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  b  Float    [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type Boolean, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  b  Float    @test.Long
        [1;94m13 | [0m  d  Boolean  [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type String, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  d  Boolean  @test.Long
        [1;94m14 | [0m  e  String   [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type DateTime, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  e  String   @test.Long
        [1;94m15 | [0m  f  DateTime [1;91m@test.Long[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Long is not compatible with declared field type Bytes, expected field type Int or BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  f  DateTime @test.Long
        [1;94m16 | [0m  g  Bytes    [1;91m@test.Long[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_int_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  BigInt   @test.Int
          b  Float    @test.Int
          d  Boolean  @test.Int
          e  String   @test.Int
          f  DateTime @test.Int
          g  Bytes    @test.Int
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type BigInt, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  BigInt   [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type Float, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  BigInt   @test.Int
        [1;94m14 | [0m  b  Float    [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type Boolean, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.Int
        [1;94m15 | [0m  d  Boolean  [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type String, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Boolean  @test.Int
        [1;94m16 | [0m  e  String   [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type DateTime, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  e  String   @test.Int
        [1;94m17 | [0m  f  DateTime [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type Bytes, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  f  DateTime @test.Int
        [1;94m18 | [0m  g  Bytes    [1;91m@test.Int[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_int_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  BigInt   @test.Int
          b  Float    @test.Int
          d  Boolean  @test.Int
          e  String   @test.Int
          f  DateTime @test.Int
          g  Bytes    @test.Int
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type BigInt, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  BigInt   [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type Float, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  BigInt   @test.Int
        [1;94m13 | [0m  b  Float    [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type Boolean, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.Int
        [1;94m14 | [0m  d  Boolean  [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type String, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Boolean  @test.Int
        [1;94m15 | [0m  e  String   [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type DateTime, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  e  String   @test.Int
        [1;94m16 | [0m  f  DateTime [1;91m@test.Int[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Int is not compatible with declared field type Bytes, expected field type Int.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  DateTime @test.Int
        [1;94m17 | [0m  g  Bytes    [1;91m@test.Int[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_bindata_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  BigInt   @test.BinData
          b  Float    @test.BinData
          d  Boolean  @test.BinData
          e  String   @test.BinData
          f  DateTime @test.BinData
          g  Int      @test.BinData
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type BigInt, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  BigInt   [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type Float, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  BigInt   @test.BinData
        [1;94m14 | [0m  b  Float    [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type Boolean, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.BinData
        [1;94m15 | [0m  d  Boolean  [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type String, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Boolean  @test.BinData
        [1;94m16 | [0m  e  String   [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type DateTime, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  e  String   @test.BinData
        [1;94m17 | [0m  f  DateTime [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type Int, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  f  DateTime @test.BinData
        [1;94m18 | [0m  g  Int      [1;91m@test.BinData[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_bindata_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  BigInt   @test.BinData
          b  Float    @test.BinData
          d  Boolean  @test.BinData
          e  String   @test.BinData
          f  DateTime @test.BinData
          g  Int      @test.BinData
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type BigInt, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  BigInt   [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type Float, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  BigInt   @test.BinData
        [1;94m13 | [0m  b  Float    [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type Boolean, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.BinData
        [1;94m14 | [0m  d  Boolean  [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type String, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Boolean  @test.BinData
        [1;94m15 | [0m  e  String   [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type DateTime, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  e  String   @test.BinData
        [1;94m16 | [0m  f  DateTime [1;91m@test.BinData[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BinData is not compatible with declared field type Int, expected field type Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  DateTime @test.BinData
        [1;94m17 | [0m  g  Int      [1;91m@test.BinData[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_object_id_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  BigInt   @test.ObjectID
          b  Float    @test.ObjectID
          d  Boolean  @test.ObjectID
          f  DateTime @test.ObjectID
          g  Int      @test.ObjectID
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type ObjectID is not supported for mongodb connector.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  BigInt   [1;91m@test.ObjectID[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectID is not supported for mongodb connector.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  BigInt   @test.ObjectID
        [1;94m14 | [0m  b  Float    [1;91m@test.ObjectID[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectID is not supported for mongodb connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.ObjectID
        [1;94m15 | [0m  d  Boolean  [1;91m@test.ObjectID[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectID is not supported for mongodb connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Boolean  @test.ObjectID
        [1;94m16 | [0m  f  DateTime [1;91m@test.ObjectID[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectID is not supported for mongodb connector.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  DateTime @test.ObjectID
        [1;94m17 | [0m  g  Int      [1;91m@test.ObjectID[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_object_id_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  BigInt   @test.ObjectId
          b  Float    @test.ObjectId
          d  Boolean  @test.ObjectId
          f  DateTime @test.ObjectId
          g  Int      @test.ObjectId
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type ObjectId is not compatible with declared field type BigInt, expected field type String or Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  BigInt   [1;91m@test.ObjectId[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectId is not compatible with declared field type Float, expected field type String or Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  BigInt   @test.ObjectId
        [1;94m13 | [0m  b  Float    [1;91m@test.ObjectId[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectId is not compatible with declared field type Boolean, expected field type String or Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.ObjectId
        [1;94m14 | [0m  d  Boolean  [1;91m@test.ObjectId[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectId is not compatible with declared field type DateTime, expected field type String or Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Boolean  @test.ObjectId
        [1;94m15 | [0m  f  DateTime [1;91m@test.ObjectId[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type ObjectId is not compatible with declared field type Int, expected field type String or Bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  f  DateTime @test.ObjectId
        [1;94m16 | [0m  g  Int      [1;91m@test.ObjectId[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_bool_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  BigInt   @test.Bool
          b  Float    @test.Bool
          d  Bytes    @test.Bool
          e  String   @test.Bool
          f  DateTime @test.Bool
          g  Int      @test.Bool
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type BigInt, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  BigInt   [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type Float, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  BigInt   @test.Bool
        [1;94m14 | [0m  b  Float    [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type Bytes, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.Bool
        [1;94m15 | [0m  d  Bytes    [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type String, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Bytes    @test.Bool
        [1;94m16 | [0m  e  String   [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type DateTime, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  e  String   @test.Bool
        [1;94m17 | [0m  f  DateTime [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type Int, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  f  DateTime @test.Bool
        [1;94m18 | [0m  g  Int      [1;91m@test.Bool[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_bool_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  BigInt   @test.Bool
          b  Float    @test.Bool
          d  Bytes    @test.Bool
          e  String   @test.Bool
          f  DateTime @test.Bool
          g  Int      @test.Bool
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type BigInt, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  BigInt   [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type Float, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  BigInt   @test.Bool
        [1;94m13 | [0m  b  Float    [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type Bytes, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.Bool
        [1;94m14 | [0m  d  Bytes    [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type String, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Bytes    @test.Bool
        [1;94m15 | [0m  e  String   [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type DateTime, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  e  String   @test.Bool
        [1;94m16 | [0m  f  DateTime [1;91m@test.Bool[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Bool is not compatible with declared field type Int, expected field type Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  DateTime @test.Bool
        [1;94m17 | [0m  g  Int      [1;91m@test.Bool[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_date_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  BigInt   @test.Date
          b  Float    @test.Date
          d  Bytes    @test.Date
          e  String   @test.Date
          f  Boolean  @test.Date
          g  Int      @test.Date
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type BigInt, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  BigInt   [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Float, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  BigInt   @test.Date
        [1;94m14 | [0m  b  Float    [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Bytes, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.Date
        [1;94m15 | [0m  d  Bytes    [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type String, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Bytes    @test.Date
        [1;94m16 | [0m  e  String   [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Boolean, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  e  String   @test.Date
        [1;94m17 | [0m  f  Boolean  [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Int, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  f  Boolean  @test.Date
        [1;94m18 | [0m  g  Int      [1;91m@test.Date[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_date_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  BigInt   @test.Date
          b  Float    @test.Date
          d  Bytes    @test.Date
          e  String   @test.Date
          f  Boolean  @test.Date
          g  Int      @test.Date
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type BigInt, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  BigInt   [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Float, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  BigInt   @test.Date
        [1;94m13 | [0m  b  Float    [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Bytes, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.Date
        [1;94m14 | [0m  d  Bytes    [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type String, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Bytes    @test.Date
        [1;94m15 | [0m  e  String   [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Boolean, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  e  String   @test.Date
        [1;94m16 | [0m  f  Boolean  [1;91m@test.Date[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Date is not compatible with declared field type Int, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  Boolean  @test.Date
        [1;94m17 | [0m  g  Int      [1;91m@test.Date[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_timestamp_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  BigInt   @test.Timestamp
          b  Float    @test.Timestamp
          d  Bytes    @test.Timestamp
          e  String   @test.Timestamp
          f  Boolean  @test.Timestamp
          g  Int      @test.Timestamp
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type BigInt, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  BigInt   [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Float, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  BigInt   @test.Timestamp
        [1;94m14 | [0m  b  Float    [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Bytes, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.Timestamp
        [1;94m15 | [0m  d  Bytes    [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type String, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Bytes    @test.Timestamp
        [1;94m16 | [0m  e  String   [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Boolean, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  e  String   @test.Timestamp
        [1;94m17 | [0m  f  Boolean  [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Int, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  f  Boolean  @test.Timestamp
        [1;94m18 | [0m  g  Int      [1;91m@test.Timestamp[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_timestamp_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  BigInt   @test.Timestamp
          b  Float    @test.Timestamp
          d  Bytes    @test.Timestamp
          e  String   @test.Timestamp
          f  Boolean  @test.Timestamp
          g  Int      @test.Timestamp
        }

        model A {
          id Int @id          @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type BigInt, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  BigInt   [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Float, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  BigInt   @test.Timestamp
        [1;94m13 | [0m  b  Float    [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Bytes, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.Timestamp
        [1;94m14 | [0m  d  Bytes    [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type String, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  d  Bytes    @test.Timestamp
        [1;94m15 | [0m  e  String   [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Boolean, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  e  String   @test.Timestamp
        [1;94m16 | [0m  f  Boolean  [1;91m@test.Timestamp[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Timestamp is not compatible with declared field type Int, expected field type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  f  Boolean  @test.Timestamp
        [1;94m17 | [0m  g  Int      [1;91m@test.Timestamp[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_json_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int      @id          @map("_id")
          a  Int      @test.Json
          b  Float    @test.Json
          c  Bytes    @test.Json
          d  Boolean  @test.Json
          e  DateTime @test.Json
          f  Decimal  @test.Json
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Int, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int      @id          @map("_id")
        [1;94m13 | [0m  a  Int      [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Float, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  a  Int      @test.Json
        [1;94m14 | [0m  b  Float    [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Bytes, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  b  Float    @test.Json
        [1;94m15 | [0m  c  Bytes    [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Boolean, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  c  Bytes    @test.Json
        [1;94m16 | [0m  d  Boolean  [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type DateTime, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  d  Boolean  @test.Json
        [1;94m17 | [0m  e  DateTime [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating field `f` in model `A`: Field `f` in model `A` can't be of type Decimal. The current connector does not support the Decimal type.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  e  DateTime @test.Json
        [1;94m18 | [0m  [1;91mf  Decimal  @test.Json[0m
        [1;94m19 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Decimal, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  e  DateTime @test.Json
        [1;94m18 | [0m  f  Decimal  [1;91m@test.Json[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn invalid_json_usage_in_type() {
    let dml = indoc! {r#"
        type B {
          a  Int      @test.Json
          b  Float    @test.Json
          c  Bytes    @test.Json
          d  Boolean  @test.Json
          e  DateTime @test.Json
          f  Decimal  @test.Json
        }

        model A {
          id Int @id @map("_id")
          b  B
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Int, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype B {
        [1;94m12 | [0m  a  Int      [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Float, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  a  Int      @test.Json
        [1;94m13 | [0m  b  Float    [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Bytes, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b  Float    @test.Json
        [1;94m14 | [0m  c  Bytes    [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Boolean, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  c  Bytes    @test.Json
        [1;94m15 | [0m  d  Boolean  [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type DateTime, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  d  Boolean  @test.Json
        [1;94m16 | [0m  e  DateTime [1;91m@test.Json[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Json is not compatible with declared field type Decimal, expected field type Json.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  e  DateTime @test.Json
        [1;94m17 | [0m  f  Decimal  [1;91m@test.Json[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}
