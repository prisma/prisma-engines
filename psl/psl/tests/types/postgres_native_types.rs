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
          geom_01 Geometry @db.Geometry(Geometry, 4326)
          geom_02 Geometry @db.Geometry(GeometryZ, 4326)
          geom_03 Geometry @db.Geometry(GeometryM, 4326)
          geom_04 Geometry @db.Geometry(GeometryZM, 4326)
          geom_05 Geometry @db.Geometry(Point, 4326)
          geom_06 Geometry @db.Geometry(PointZ, 4326)
          geom_07 Geometry @db.Geometry(PointM, 4326)
          geom_08 Geometry @db.Geometry(PointZM, 4326)
          geom_09 Geometry @db.Geometry(LineString, 4326)
          geom_10 Geometry @db.Geometry(LineStringZ, 4326)
          geom_11 Geometry @db.Geometry(LineStringM, 4326)
          geom_12 Geometry @db.Geometry(LineStringZM, 4326)
          geom_13 Geometry @db.Geometry(Polygon, 4326)
          geom_14 Geometry @db.Geometry(PolygonZ, 4326)
          geom_15 Geometry @db.Geometry(PolygonM, 4326)
          geom_16 Geometry @db.Geometry(PolygonZM, 4326)
          geom_17 Geometry @db.Geometry(MultiPoint, 4326)
          geom_18 Geometry @db.Geometry(MultiPointZ, 4326)
          geom_19 Geometry @db.Geometry(MultiPointM, 4326)
          geom_20 Geometry @db.Geometry(MultiPointZM, 4326)
          geom_21 Geometry @db.Geometry(MultiLineString, 4326)
          geom_22 Geometry @db.Geometry(MultiLineStringZ, 4326)
          geom_23 Geometry @db.Geometry(MultiLineStringM, 4326)
          geom_24 Geometry @db.Geometry(MultiLineStringZM, 4326)
          geom_25 Geometry @db.Geometry(MultiPolygon, 4326)
          geom_26 Geometry @db.Geometry(MultiPolygonZ, 4326)
          geom_27 Geometry @db.Geometry(MultiPolygonM, 4326)
          geom_28 Geometry @db.Geometry(MultiPolygonZM, 4326)
          geom_29 Geometry @db.Geometry(GeometryCollection, 4326)
          geom_30 Geometry @db.Geometry(GeometryCollectionZ, 4326)
          geom_31 Geometry @db.Geometry(GeometryCollectionM, 4326)
          geom_32 Geometry @db.Geometry(GeometryCollectionZM, 4326)
          geog_01 Geometry @db.Geography(Geometry, 4326)
          geog_02 Geometry @db.Geography(GeometryZ, 4326)
          geog_03 Geometry @db.Geography(GeometryM, 4326)
          geog_04 Geometry @db.Geography(GeometryZM, 4326)
          geog_05 Geometry @db.Geography(Point, 4326)
          geog_06 Geometry @db.Geography(PointZ, 4326)
          geog_07 Geometry @db.Geography(PointM, 4326)
          geog_08 Geometry @db.Geography(PointZM, 4326)
          geog_09 Geometry @db.Geography(LineString, 4326)
          geog_10 Geometry @db.Geography(LineStringZ, 4326)
          geog_11 Geometry @db.Geography(LineStringM, 4326)
          geog_12 Geometry @db.Geography(LineStringZM, 4326)
          geog_13 Geometry @db.Geography(Polygon, 4326)
          geog_14 Geometry @db.Geography(PolygonZ, 4326)
          geog_15 Geometry @db.Geography(PolygonM, 4326)
          geog_16 Geometry @db.Geography(PolygonZM, 4326)
          geog_17 Geometry @db.Geography(MultiPoint, 4326)
          geog_18 Geometry @db.Geography(MultiPointZ, 4326)
          geog_19 Geometry @db.Geography(MultiPointM, 4326)
          geog_20 Geometry @db.Geography(MultiPointZM, 4326)
          geog_21 Geometry @db.Geography(MultiLineString, 4326)
          geog_22 Geometry @db.Geography(MultiLineStringZ, 4326)
          geog_23 Geometry @db.Geography(MultiLineStringM, 4326)
          geog_24 Geometry @db.Geography(MultiLineStringZM, 4326)
          geog_25 Geometry @db.Geography(MultiPolygon, 4326)
          geog_26 Geometry @db.Geography(MultiPolygonZ, 4326)
          geog_27 Geometry @db.Geography(MultiPolygonM, 4326)
          geog_28 Geometry @db.Geography(MultiPolygonZM, 4326)
          geog_29 Geometry @db.Geography(GeometryCollection, 4326)
          geog_30 Geometry @db.Geography(GeometryCollectionZ, 4326)
          geog_31 Geometry @db.Geography(GeometryCollectionM, 4326)
          geog_32 Geometry @db.Geography(GeometryCollectionZM, 4326)
        }
    "#};

    psl::parse_schema(schema).unwrap();
}

#[test]
fn should_fail_on_geojson_when_incompatible_geometry_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = env("TEST_DATABASE_URL")
        }

        model Blog {
          id      Int     @id
          geom_01 GeoJson @db.Geometry(GeometryM, 4326)
          geom_02 GeoJson @db.Geometry(GeometryZM, 4326)
          geom_03 GeoJson @db.Geometry(PointM, 4326)
          geom_04 GeoJson @db.Geometry(PointZM, 4326)
          geom_05 GeoJson @db.Geometry(PointM, 4326)
          geom_06 GeoJson @db.Geometry(PointZM, 4326)
          geom_07 GeoJson @db.Geometry(LineStringM, 4326)
          geom_08 GeoJson @db.Geometry(LineStringZM, 4326)
          geom_09 GeoJson @db.Geometry(PolygonM, 4326)
          geom_10 GeoJson @db.Geometry(PolygonZM, 4326)
          geom_11 GeoJson @db.Geometry(MultiPointM, 4326)
          geom_12 GeoJson @db.Geometry(MultiPointZM, 4326)
          geom_13 GeoJson @db.Geometry(MultiLineStringM, 4326)
          geom_14 GeoJson @db.Geometry(MultiLineStringZM, 4326)
          geom_15 GeoJson @db.Geometry(MultiPolygonM, 4326)
          geom_16 GeoJson @db.Geometry(MultiPolygonZM, 4326)
          geom_17 GeoJson @db.Geometry(GeometryCollectionM, 4326)
          geom_18 GeoJson @db.Geometry(GeometryCollectionZM, 4326)
          geog_01 GeoJson @db.Geography(GeometryM, 4326)
          geog_02 GeoJson @db.Geography(GeometryZM, 4326)
          geog_03 GeoJson @db.Geography(PointM, 4326)
          geog_04 GeoJson @db.Geography(PointZM, 4326)
          geog_05 GeoJson @db.Geography(PointM, 4326)
          geog_06 GeoJson @db.Geography(PointZM, 4326)
          geog_07 GeoJson @db.Geography(LineStringM, 4326)
          geog_08 GeoJson @db.Geography(LineStringZM, 4326)
          geog_09 GeoJson @db.Geography(PolygonM, 4326)
          geog_10 GeoJson @db.Geography(PolygonZM, 4326)
          geog_11 GeoJson @db.Geography(MultiPointM, 4326)
          geog_12 GeoJson @db.Geography(MultiPointZM, 4326)
          geog_13 GeoJson @db.Geography(MultiLineStringM, 4326)
          geog_14 GeoJson @db.Geography(MultiLineStringZM, 4326)
          geog_15 GeoJson @db.Geography(MultiPolygonM, 4326)
          geog_16 GeoJson @db.Geography(MultiPolygonZM, 4326)
          geog_17 GeoJson @db.Geography(GeometryCollectionM, 4326)
          geog_18 GeoJson @db.Geography(GeometryCollectionZM, 4326)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryM,4326)` of Postgres: GeometryM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id      Int     @id
        [1;94m 8 | [0m  geom_01 GeoJson [1;91m@db.Geometry(GeometryM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryZM,4326)` of Postgres: GeometryZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  geom_01 GeoJson @db.Geometry(GeometryM, 4326)
        [1;94m 9 | [0m  geom_02 GeoJson [1;91m@db.Geometry(GeometryZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointM,4326)` of Postgres: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  geom_02 GeoJson @db.Geometry(GeometryZM, 4326)
        [1;94m10 | [0m  geom_03 GeoJson [1;91m@db.Geometry(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointZM,4326)` of Postgres: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m  geom_03 GeoJson @db.Geometry(PointM, 4326)
        [1;94m11 | [0m  geom_04 GeoJson [1;91m@db.Geometry(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointM,4326)` of Postgres: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  geom_04 GeoJson @db.Geometry(PointZM, 4326)
        [1;94m12 | [0m  geom_05 GeoJson [1;91m@db.Geometry(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointZM,4326)` of Postgres: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  geom_05 GeoJson @db.Geometry(PointM, 4326)
        [1;94m13 | [0m  geom_06 GeoJson [1;91m@db.Geometry(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(LineStringM,4326)` of Postgres: LineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  geom_06 GeoJson @db.Geometry(PointZM, 4326)
        [1;94m14 | [0m  geom_07 GeoJson [1;91m@db.Geometry(LineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(LineStringZM,4326)` of Postgres: LineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  geom_07 GeoJson @db.Geometry(LineStringM, 4326)
        [1;94m15 | [0m  geom_08 GeoJson [1;91m@db.Geometry(LineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolygonM,4326)` of Postgres: PolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  geom_08 GeoJson @db.Geometry(LineStringZM, 4326)
        [1;94m16 | [0m  geom_09 GeoJson [1;91m@db.Geometry(PolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolygonZM,4326)` of Postgres: PolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  geom_09 GeoJson @db.Geometry(PolygonM, 4326)
        [1;94m17 | [0m  geom_10 GeoJson [1;91m@db.Geometry(PolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPointM,4326)` of Postgres: MultiPointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  geom_10 GeoJson @db.Geometry(PolygonZM, 4326)
        [1;94m18 | [0m  geom_11 GeoJson [1;91m@db.Geometry(MultiPointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPointZM,4326)` of Postgres: MultiPointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m  geom_11 GeoJson @db.Geometry(MultiPointM, 4326)
        [1;94m19 | [0m  geom_12 GeoJson [1;91m@db.Geometry(MultiPointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiLineStringM,4326)` of Postgres: MultiLineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m  geom_12 GeoJson @db.Geometry(MultiPointZM, 4326)
        [1;94m20 | [0m  geom_13 GeoJson [1;91m@db.Geometry(MultiLineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiLineStringZM,4326)` of Postgres: MultiLineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:21[0m
        [1;94m   | [0m
        [1;94m20 | [0m  geom_13 GeoJson @db.Geometry(MultiLineStringM, 4326)
        [1;94m21 | [0m  geom_14 GeoJson [1;91m@db.Geometry(MultiLineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPolygonM,4326)` of Postgres: MultiPolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:22[0m
        [1;94m   | [0m
        [1;94m21 | [0m  geom_14 GeoJson @db.Geometry(MultiLineStringZM, 4326)
        [1;94m22 | [0m  geom_15 GeoJson [1;91m@db.Geometry(MultiPolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPolygonZM,4326)` of Postgres: MultiPolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:23[0m
        [1;94m   | [0m
        [1;94m22 | [0m  geom_15 GeoJson @db.Geometry(MultiPolygonM, 4326)
        [1;94m23 | [0m  geom_16 GeoJson [1;91m@db.Geometry(MultiPolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryCollectionM,4326)` of Postgres: GeometryCollectionM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:24[0m
        [1;94m   | [0m
        [1;94m23 | [0m  geom_16 GeoJson @db.Geometry(MultiPolygonZM, 4326)
        [1;94m24 | [0m  geom_17 GeoJson [1;91m@db.Geometry(GeometryCollectionM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryCollectionZM,4326)` of Postgres: GeometryCollectionZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:25[0m
        [1;94m   | [0m
        [1;94m24 | [0m  geom_17 GeoJson @db.Geometry(GeometryCollectionM, 4326)
        [1;94m25 | [0m  geom_18 GeoJson [1;91m@db.Geometry(GeometryCollectionZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryM,4326)` of Postgres: GeometryM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:26[0m
        [1;94m   | [0m
        [1;94m25 | [0m  geom_18 GeoJson @db.Geometry(GeometryCollectionZM, 4326)
        [1;94m26 | [0m  geog_01 GeoJson [1;91m@db.Geography(GeometryM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryZM,4326)` of Postgres: GeometryZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:27[0m
        [1;94m   | [0m
        [1;94m26 | [0m  geog_01 GeoJson @db.Geography(GeometryM, 4326)
        [1;94m27 | [0m  geog_02 GeoJson [1;91m@db.Geography(GeometryZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointM,4326)` of Postgres: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:28[0m
        [1;94m   | [0m
        [1;94m27 | [0m  geog_02 GeoJson @db.Geography(GeometryZM, 4326)
        [1;94m28 | [0m  geog_03 GeoJson [1;91m@db.Geography(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointZM,4326)` of Postgres: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:29[0m
        [1;94m   | [0m
        [1;94m28 | [0m  geog_03 GeoJson @db.Geography(PointM, 4326)
        [1;94m29 | [0m  geog_04 GeoJson [1;91m@db.Geography(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointM,4326)` of Postgres: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:30[0m
        [1;94m   | [0m
        [1;94m29 | [0m  geog_04 GeoJson @db.Geography(PointZM, 4326)
        [1;94m30 | [0m  geog_05 GeoJson [1;91m@db.Geography(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointZM,4326)` of Postgres: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:31[0m
        [1;94m   | [0m
        [1;94m30 | [0m  geog_05 GeoJson @db.Geography(PointM, 4326)
        [1;94m31 | [0m  geog_06 GeoJson [1;91m@db.Geography(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(LineStringM,4326)` of Postgres: LineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:32[0m
        [1;94m   | [0m
        [1;94m31 | [0m  geog_06 GeoJson @db.Geography(PointZM, 4326)
        [1;94m32 | [0m  geog_07 GeoJson [1;91m@db.Geography(LineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(LineStringZM,4326)` of Postgres: LineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:33[0m
        [1;94m   | [0m
        [1;94m32 | [0m  geog_07 GeoJson @db.Geography(LineStringM, 4326)
        [1;94m33 | [0m  geog_08 GeoJson [1;91m@db.Geography(LineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolygonM,4326)` of Postgres: PolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:34[0m
        [1;94m   | [0m
        [1;94m33 | [0m  geog_08 GeoJson @db.Geography(LineStringZM, 4326)
        [1;94m34 | [0m  geog_09 GeoJson [1;91m@db.Geography(PolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolygonZM,4326)` of Postgres: PolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:35[0m
        [1;94m   | [0m
        [1;94m34 | [0m  geog_09 GeoJson @db.Geography(PolygonM, 4326)
        [1;94m35 | [0m  geog_10 GeoJson [1;91m@db.Geography(PolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPointM,4326)` of Postgres: MultiPointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:36[0m
        [1;94m   | [0m
        [1;94m35 | [0m  geog_10 GeoJson @db.Geography(PolygonZM, 4326)
        [1;94m36 | [0m  geog_11 GeoJson [1;91m@db.Geography(MultiPointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPointZM,4326)` of Postgres: MultiPointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:37[0m
        [1;94m   | [0m
        [1;94m36 | [0m  geog_11 GeoJson @db.Geography(MultiPointM, 4326)
        [1;94m37 | [0m  geog_12 GeoJson [1;91m@db.Geography(MultiPointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiLineStringM,4326)` of Postgres: MultiLineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:38[0m
        [1;94m   | [0m
        [1;94m37 | [0m  geog_12 GeoJson @db.Geography(MultiPointZM, 4326)
        [1;94m38 | [0m  geog_13 GeoJson [1;91m@db.Geography(MultiLineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiLineStringZM,4326)` of Postgres: MultiLineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:39[0m
        [1;94m   | [0m
        [1;94m38 | [0m  geog_13 GeoJson @db.Geography(MultiLineStringM, 4326)
        [1;94m39 | [0m  geog_14 GeoJson [1;91m@db.Geography(MultiLineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPolygonM,4326)` of Postgres: MultiPolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:40[0m
        [1;94m   | [0m
        [1;94m39 | [0m  geog_14 GeoJson @db.Geography(MultiLineStringZM, 4326)
        [1;94m40 | [0m  geog_15 GeoJson [1;91m@db.Geography(MultiPolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPolygonZM,4326)` of Postgres: MultiPolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:41[0m
        [1;94m   | [0m
        [1;94m40 | [0m  geog_15 GeoJson @db.Geography(MultiPolygonM, 4326)
        [1;94m41 | [0m  geog_16 GeoJson [1;91m@db.Geography(MultiPolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryCollectionM,4326)` of Postgres: GeometryCollectionM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:42[0m
        [1;94m   | [0m
        [1;94m41 | [0m  geog_16 GeoJson @db.Geography(MultiPolygonZM, 4326)
        [1;94m42 | [0m  geog_17 GeoJson [1;91m@db.Geography(GeometryCollectionM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryCollectionZM,4326)` of Postgres: GeometryCollectionZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:43[0m
        [1;94m   | [0m
        [1;94m42 | [0m  geog_17 GeoJson @db.Geography(GeometryCollectionM, 4326)
        [1;94m43 | [0m  geog_18 GeoJson [1;91m@db.Geography(GeometryCollectionZM, 4326)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}
