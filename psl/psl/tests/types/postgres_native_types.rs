use crate::common::*;
use psl::builtin_connectors::PostgresType;

#[test]
fn xml_data_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Xml
          lastName  String @db.Xml

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `Xml` of Postgres.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int    @id
        [1;94m 8 | [0m  [1;91mfirstName String @db.Xml[0m
        [1;94m 9 | [0m  lastName  String @db.Xml
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn xml_data_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Xml
          lastName  String @db.Xml

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Xml` cannot be unique in Postgres.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int    @id
        [1;94m 8 | [0m  [1;91mfirstName String @db.Xml[0m
        [1;94m 9 | [0m  lastName  String @db.Xml
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_invalid_precision_for_decimal_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int     @id
          val Decimal @db.Decimal(1001,3)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(1001,3)` of Postgres: Precision must be positive with a maximum value of 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int     @id
        [1;94m 8 | [0m  val Decimal [1;91m@db.Decimal(1001,3)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_invalid_precision_for_time_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Time(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Time(7)` of Postgres: M can range from 0 to 6.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int      @id
        [1;94m 8 | [0m  val DateTime [1;91m@db.Time(7)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Timestamp(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Timestamp(7)` of Postgres: M can range from 0 to 6.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int      @id
        [1;94m 8 | [0m  val DateTime [1;91m@db.Timestamp(7)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_data_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int   @id
          val Bytes @db.Bit(0)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type Bit is not compatible with declared field type Bytes, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int   @id
        [1;94m 8 | [0m  val Bytes [1;91m@db.Bit(0)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int   @id
          val Bytes @db.VarBit(0)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type VarBit is not compatible with declared field type Bytes, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int   @id
        [1;94m 8 | [0m  val Bytes [1;91m@db.VarBit(0)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id     Int   @id
          dec Decimal @db.Decimal(2, 4)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in Postgres.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int   @id
        [1;94m 8 | [0m  dec Decimal [1;91m@db.Decimal(2, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn xml_should_work_with_string_scalar_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int    @id
          dec String @db.Xml
        }
    "#};

    let datamodel = psl::parse_schema(dml).unwrap();
    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_scalar_field("dec")
        .assert_native_type(datamodel.connector, &PostgresType::Xml);
}

#[test]
fn postgis_specific_native_types_are_valid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = env("TEST_DATABASE_URL")
        }

        model NativeTypesTest {
          id      Int      @id
          geom_01 Geometry
          geom_02 Geometry @db.Geometry(GeometryZ, 4326)
          geom_03 Geometry @db.Geometry(Point, 4326)
          geom_04 Geometry @db.Geometry(PointZ, 4326)
          geom_05 Geometry @db.Geometry(LineString, 4326)
          geom_06 Geometry @db.Geometry(LineStringZ, 4326)
          geom_07 Geometry @db.Geometry(Polygon, 4326)
          geom_08 Geometry @db.Geometry(PolygonZ, 4326)
          geom_09 Geometry @db.Geometry(MultiPoint, 4326)
          geom_10 Geometry @db.Geometry(MultiPointZ, 4326)
          geom_11 Geometry @db.Geometry(MultiLineString, 4326)
          geom_12 Geometry @db.Geometry(MultiLineStringZ, 4326)
          geom_13 Geometry @db.Geometry(MultiPolygon, 4326)
          geom_14 Geometry @db.Geometry(MultiPolygonZ, 4326)
          geom_15 Geometry @db.Geometry(GeometryCollection, 4326)
          geom_16 Geometry @db.Geometry(GeometryCollectionZ, 4326)
          geog_01 Geometry @db.Geography(Geometry, 4326)
          geog_02 Geometry @db.Geography(GeometryZ, 4326)
          geog_03 Geometry @db.Geography(Point, 4326)
          geog_04 Geometry @db.Geography(PointZ, 4326)
          geog_05 Geometry @db.Geography(LineString, 4326)
          geog_06 Geometry @db.Geography(LineStringZ, 4326)
          geog_07 Geometry @db.Geography(Polygon, 4326)
          geog_08 Geometry @db.Geography(PolygonZ, 4326)
          geog_09 Geometry @db.Geography(MultiPoint, 4326)
          geog_10 Geometry @db.Geography(MultiPointZ, 4326)
          geog_11 Geometry @db.Geography(MultiLineString, 4326)
          geog_12 Geometry @db.Geography(MultiLineStringZ, 4326)
          geog_13 Geometry @db.Geography(MultiPolygon, 4326)
          geog_14 Geometry @db.Geography(MultiPolygonZ, 4326)
          geog_15 Geometry @db.Geography(GeometryCollection, 4326)
          geog_16 Geometry @db.Geography(GeometryCollectionZ, 4326)
        }
    "#};

    psl::parse_schema(schema).unwrap();
}
