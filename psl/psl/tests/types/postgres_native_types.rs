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
          geom_33 Geometry @db.Geometry(CircularString, 4326)
          geom_34 Geometry @db.Geometry(CircularStringZ, 4326)
          geom_35 Geometry @db.Geometry(CircularStringM, 4326)
          geom_36 Geometry @db.Geometry(CircularStringZM, 4326)
          geom_37 Geometry @db.Geometry(CompoundCurve, 4326)
          geom_38 Geometry @db.Geometry(CompoundCurveZ, 4326)
          geom_39 Geometry @db.Geometry(CompoundCurveM, 4326)
          geom_40 Geometry @db.Geometry(CompoundCurveZM, 4326)
          geom_41 Geometry @db.Geometry(CurvePolygon, 4326)
          geom_42 Geometry @db.Geometry(CurvePolygonZ, 4326)
          geom_43 Geometry @db.Geometry(CurvePolygonM, 4326)
          geom_44 Geometry @db.Geometry(CurvePolygonZM, 4326)
          geom_45 Geometry @db.Geometry(MultiCurve, 4326)
          geom_46 Geometry @db.Geometry(MultiCurveZ, 4326)
          geom_47 Geometry @db.Geometry(MultiCurveM, 4326)
          geom_48 Geometry @db.Geometry(MultiCurveZM, 4326)
          geom_49 Geometry @db.Geometry(MultiSurface, 4326)
          geom_50 Geometry @db.Geometry(MultiSurfaceZ, 4326)
          geom_51 Geometry @db.Geometry(MultiSurfaceM, 4326)
          geom_52 Geometry @db.Geometry(MultiSurfaceZM, 4326)
          geom_53 Geometry @db.Geometry(PolyhedralSurface, 4326)
          geom_54 Geometry @db.Geometry(PolyhedralSurfaceZ, 4326)
          geom_55 Geometry @db.Geometry(PolyhedralSurfaceM, 4326)
          geom_56 Geometry @db.Geometry(PolyhedralSurfaceZM, 4326)
          geom_57 Geometry @db.Geometry(Tin, 4326)
          geom_58 Geometry @db.Geometry(TinZ, 4326)
          geom_59 Geometry @db.Geometry(TinM, 4326)
          geom_60 Geometry @db.Geometry(TinZM, 4326)
          geom_61 Geometry @db.Geometry(Triangle, 4326)
          geom_62 Geometry @db.Geometry(TriangleZ, 4326)
          geom_63 Geometry @db.Geometry(TriangleM, 4326)
          geom_64 Geometry @db.Geometry(TriangleZM, 4326)
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
          geog_33 Geometry @db.Geography(CircularString, 4326)
          geog_34 Geometry @db.Geography(CircularStringZ, 4326)
          geog_35 Geometry @db.Geography(CircularStringM, 4326)
          geog_36 Geometry @db.Geography(CircularStringZM, 4326)
          geog_37 Geometry @db.Geography(CompoundCurve, 4326)
          geog_38 Geometry @db.Geography(CompoundCurveZ, 4326)
          geog_39 Geometry @db.Geography(CompoundCurveM, 4326)
          geog_40 Geometry @db.Geography(CompoundCurveZM, 4326)
          geog_41 Geometry @db.Geography(CurvePolygon, 4326)
          geog_42 Geometry @db.Geography(CurvePolygonZ, 4326)
          geog_43 Geometry @db.Geography(CurvePolygonM, 4326)
          geog_44 Geometry @db.Geography(CurvePolygonZM, 4326)
          geog_45 Geometry @db.Geography(MultiCurve, 4326)
          geog_46 Geometry @db.Geography(MultiCurveZ, 4326)
          geog_47 Geometry @db.Geography(MultiCurveM, 4326)
          geog_48 Geometry @db.Geography(MultiCurveZM, 4326)
          geog_49 Geometry @db.Geography(MultiSurface, 4326)
          geog_50 Geometry @db.Geography(MultiSurfaceZ, 4326)
          geog_51 Geometry @db.Geography(MultiSurfaceM, 4326)
          geog_52 Geometry @db.Geography(MultiSurfaceZM, 4326)
          geog_53 Geometry @db.Geography(PolyhedralSurface, 4326)
          geog_54 Geometry @db.Geography(PolyhedralSurfaceZ, 4326)
          geog_55 Geometry @db.Geography(PolyhedralSurfaceM, 4326)
          geog_56 Geometry @db.Geography(PolyhedralSurfaceZM, 4326)
          geog_57 Geometry @db.Geography(Tin, 4326)
          geog_58 Geometry @db.Geography(TinZ, 4326)
          geog_59 Geometry @db.Geography(TinM, 4326)
          geog_60 Geometry @db.Geography(TinZM, 4326)
          geog_61 Geometry @db.Geography(Triangle, 4326)
          geog_62 Geometry @db.Geography(TriangleZ, 4326)
          geog_63 Geometry @db.Geography(TriangleM, 4326)
          geog_64 Geometry @db.Geography(TriangleZM, 4326)
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
          geom_19 GeoJson @db.Geometry(CircularString, 4326)
          geom_20 GeoJson @db.Geometry(CircularStringZ, 4326)
          geom_21 GeoJson @db.Geometry(CircularStringM, 4326)
          geom_22 GeoJson @db.Geometry(CircularStringZM, 4326)
          geom_23 GeoJson @db.Geometry(CompoundCurve, 4326)
          geom_24 GeoJson @db.Geometry(CompoundCurveZ, 4326)
          geom_35 GeoJson @db.Geometry(CompoundCurveM, 4326)
          geom_36 GeoJson @db.Geometry(CompoundCurveZM, 4326)
          geom_37 GeoJson @db.Geometry(CurvePolygon, 4326)
          geom_38 GeoJson @db.Geometry(CurvePolygonZ, 4326)
          geom_39 GeoJson @db.Geometry(CurvePolygonM, 4326)
          geom_40 GeoJson @db.Geometry(CurvePolygonZM, 4326)
          geom_41 GeoJson @db.Geometry(MultiCurve, 4326)
          geom_42 GeoJson @db.Geometry(MultiCurveZ, 4326)
          geom_43 GeoJson @db.Geometry(MultiCurveM, 4326)
          geom_44 GeoJson @db.Geometry(MultiCurveZM, 4326)
          geom_45 GeoJson @db.Geometry(MultiSurface, 4326)
          geom_46 GeoJson @db.Geometry(MultiSurfaceZ, 4326)
          geom_47 GeoJson @db.Geometry(MultiSurfaceM, 4326)
          geom_48 GeoJson @db.Geometry(MultiSurfaceZM, 4326)
          geom_49 GeoJson @db.Geometry(PolyhedralSurface, 4326)
          geom_50 GeoJson @db.Geometry(PolyhedralSurfaceZ, 4326)
          geom_51 GeoJson @db.Geometry(PolyhedralSurfaceM, 4326)
          geom_52 GeoJson @db.Geometry(PolyhedralSurfaceZM, 4326)
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
          geog_19 GeoJson @db.Geography(CircularString, 4326)
          geog_20 GeoJson @db.Geography(CircularStringZ, 4326)
          geog_21 GeoJson @db.Geography(CircularStringM, 4326)
          geog_22 GeoJson @db.Geography(CircularStringZM, 4326)
          geog_23 GeoJson @db.Geography(CompoundCurve, 4326)
          geog_24 GeoJson @db.Geography(CompoundCurveZ, 4326)
          geog_35 GeoJson @db.Geography(CompoundCurveM, 4326)
          geog_36 GeoJson @db.Geography(CompoundCurveZM, 4326)
          geog_37 GeoJson @db.Geography(CurvePolygon, 4326)
          geog_38 GeoJson @db.Geography(CurvePolygonZ, 4326)
          geog_39 GeoJson @db.Geography(CurvePolygonM, 4326)
          geog_40 GeoJson @db.Geography(CurvePolygonZM, 4326)
          geog_41 GeoJson @db.Geography(MultiCurve, 4326)
          geog_42 GeoJson @db.Geography(MultiCurveZ, 4326)
          geog_43 GeoJson @db.Geography(MultiCurveM, 4326)
          geog_44 GeoJson @db.Geography(MultiCurveZM, 4326)
          geog_45 GeoJson @db.Geography(MultiSurface, 4326)
          geog_46 GeoJson @db.Geography(MultiSurfaceZ, 4326)
          geog_47 GeoJson @db.Geography(MultiSurfaceM, 4326)
          geog_48 GeoJson @db.Geography(MultiSurfaceZM, 4326)
          geog_49 GeoJson @db.Geography(PolyhedralSurface, 4326)
          geog_50 GeoJson @db.Geography(PolyhedralSurfaceZ, 4326)
          geog_51 GeoJson @db.Geography(PolyhedralSurfaceM, 4326)
          geog_52 GeoJson @db.Geography(PolyhedralSurfaceZM, 4326)
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
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularString,4326)` of Postgres: CircularString isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:26[0m
        [1;94m   | [0m
        [1;94m25 | [0m  geom_18 GeoJson @db.Geometry(GeometryCollectionZM, 4326)
        [1;94m26 | [0m  geom_19 GeoJson [1;91m@db.Geometry(CircularString, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularStringZ,4326)` of Postgres: CircularStringZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:27[0m
        [1;94m   | [0m
        [1;94m26 | [0m  geom_19 GeoJson @db.Geometry(CircularString, 4326)
        [1;94m27 | [0m  geom_20 GeoJson [1;91m@db.Geometry(CircularStringZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularStringM,4326)` of Postgres: CircularStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:28[0m
        [1;94m   | [0m
        [1;94m27 | [0m  geom_20 GeoJson @db.Geometry(CircularStringZ, 4326)
        [1;94m28 | [0m  geom_21 GeoJson [1;91m@db.Geometry(CircularStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularStringZM,4326)` of Postgres: CircularStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:29[0m
        [1;94m   | [0m
        [1;94m28 | [0m  geom_21 GeoJson @db.Geometry(CircularStringM, 4326)
        [1;94m29 | [0m  geom_22 GeoJson [1;91m@db.Geometry(CircularStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurve,4326)` of Postgres: CompoundCurve isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:30[0m
        [1;94m   | [0m
        [1;94m29 | [0m  geom_22 GeoJson @db.Geometry(CircularStringZM, 4326)
        [1;94m30 | [0m  geom_23 GeoJson [1;91m@db.Geometry(CompoundCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurveZ,4326)` of Postgres: CompoundCurveZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:31[0m
        [1;94m   | [0m
        [1;94m30 | [0m  geom_23 GeoJson @db.Geometry(CompoundCurve, 4326)
        [1;94m31 | [0m  geom_24 GeoJson [1;91m@db.Geometry(CompoundCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurveM,4326)` of Postgres: CompoundCurveM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:32[0m
        [1;94m   | [0m
        [1;94m31 | [0m  geom_24 GeoJson @db.Geometry(CompoundCurveZ, 4326)
        [1;94m32 | [0m  geom_35 GeoJson [1;91m@db.Geometry(CompoundCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurveZM,4326)` of Postgres: CompoundCurveZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:33[0m
        [1;94m   | [0m
        [1;94m32 | [0m  geom_35 GeoJson @db.Geometry(CompoundCurveM, 4326)
        [1;94m33 | [0m  geom_36 GeoJson [1;91m@db.Geometry(CompoundCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygon,4326)` of Postgres: CurvePolygon isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:34[0m
        [1;94m   | [0m
        [1;94m33 | [0m  geom_36 GeoJson @db.Geometry(CompoundCurveZM, 4326)
        [1;94m34 | [0m  geom_37 GeoJson [1;91m@db.Geometry(CurvePolygon, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygonZ,4326)` of Postgres: CurvePolygonZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:35[0m
        [1;94m   | [0m
        [1;94m34 | [0m  geom_37 GeoJson @db.Geometry(CurvePolygon, 4326)
        [1;94m35 | [0m  geom_38 GeoJson [1;91m@db.Geometry(CurvePolygonZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygonM,4326)` of Postgres: CurvePolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:36[0m
        [1;94m   | [0m
        [1;94m35 | [0m  geom_38 GeoJson @db.Geometry(CurvePolygonZ, 4326)
        [1;94m36 | [0m  geom_39 GeoJson [1;91m@db.Geometry(CurvePolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygonZM,4326)` of Postgres: CurvePolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:37[0m
        [1;94m   | [0m
        [1;94m36 | [0m  geom_39 GeoJson @db.Geometry(CurvePolygonM, 4326)
        [1;94m37 | [0m  geom_40 GeoJson [1;91m@db.Geometry(CurvePolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurve,4326)` of Postgres: MultiCurve isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:38[0m
        [1;94m   | [0m
        [1;94m37 | [0m  geom_40 GeoJson @db.Geometry(CurvePolygonZM, 4326)
        [1;94m38 | [0m  geom_41 GeoJson [1;91m@db.Geometry(MultiCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurveZ,4326)` of Postgres: MultiCurveZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:39[0m
        [1;94m   | [0m
        [1;94m38 | [0m  geom_41 GeoJson @db.Geometry(MultiCurve, 4326)
        [1;94m39 | [0m  geom_42 GeoJson [1;91m@db.Geometry(MultiCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurveM,4326)` of Postgres: MultiCurveM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:40[0m
        [1;94m   | [0m
        [1;94m39 | [0m  geom_42 GeoJson @db.Geometry(MultiCurveZ, 4326)
        [1;94m40 | [0m  geom_43 GeoJson [1;91m@db.Geometry(MultiCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurveZM,4326)` of Postgres: MultiCurveZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:41[0m
        [1;94m   | [0m
        [1;94m40 | [0m  geom_43 GeoJson @db.Geometry(MultiCurveM, 4326)
        [1;94m41 | [0m  geom_44 GeoJson [1;91m@db.Geometry(MultiCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurface,4326)` of Postgres: MultiSurface isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:42[0m
        [1;94m   | [0m
        [1;94m41 | [0m  geom_44 GeoJson @db.Geometry(MultiCurveZM, 4326)
        [1;94m42 | [0m  geom_45 GeoJson [1;91m@db.Geometry(MultiSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurfaceZ,4326)` of Postgres: MultiSurfaceZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:43[0m
        [1;94m   | [0m
        [1;94m42 | [0m  geom_45 GeoJson @db.Geometry(MultiSurface, 4326)
        [1;94m43 | [0m  geom_46 GeoJson [1;91m@db.Geometry(MultiSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurfaceM,4326)` of Postgres: MultiSurfaceM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:44[0m
        [1;94m   | [0m
        [1;94m43 | [0m  geom_46 GeoJson @db.Geometry(MultiSurfaceZ, 4326)
        [1;94m44 | [0m  geom_47 GeoJson [1;91m@db.Geometry(MultiSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurfaceZM,4326)` of Postgres: MultiSurfaceZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:45[0m
        [1;94m   | [0m
        [1;94m44 | [0m  geom_47 GeoJson @db.Geometry(MultiSurfaceM, 4326)
        [1;94m45 | [0m  geom_48 GeoJson [1;91m@db.Geometry(MultiSurfaceZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurface,4326)` of Postgres: PolyhedralSurface isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:46[0m
        [1;94m   | [0m
        [1;94m45 | [0m  geom_48 GeoJson @db.Geometry(MultiSurfaceZM, 4326)
        [1;94m46 | [0m  geom_49 GeoJson [1;91m@db.Geometry(PolyhedralSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurfaceZ,4326)` of Postgres: PolyhedralSurfaceZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:47[0m
        [1;94m   | [0m
        [1;94m46 | [0m  geom_49 GeoJson @db.Geometry(PolyhedralSurface, 4326)
        [1;94m47 | [0m  geom_50 GeoJson [1;91m@db.Geometry(PolyhedralSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurfaceM,4326)` of Postgres: PolyhedralSurfaceM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:48[0m
        [1;94m   | [0m
        [1;94m47 | [0m  geom_50 GeoJson @db.Geometry(PolyhedralSurfaceZ, 4326)
        [1;94m48 | [0m  geom_51 GeoJson [1;91m@db.Geometry(PolyhedralSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurfaceZM,4326)` of Postgres: PolyhedralSurfaceZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:49[0m
        [1;94m   | [0m
        [1;94m48 | [0m  geom_51 GeoJson @db.Geometry(PolyhedralSurfaceM, 4326)
        [1;94m49 | [0m  geom_52 GeoJson [1;91m@db.Geometry(PolyhedralSurfaceZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryM,4326)` of Postgres: GeometryM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:50[0m
        [1;94m   | [0m
        [1;94m49 | [0m  geom_52 GeoJson @db.Geometry(PolyhedralSurfaceZM, 4326)
        [1;94m50 | [0m  geog_01 GeoJson [1;91m@db.Geography(GeometryM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryZM,4326)` of Postgres: GeometryZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:51[0m
        [1;94m   | [0m
        [1;94m50 | [0m  geog_01 GeoJson @db.Geography(GeometryM, 4326)
        [1;94m51 | [0m  geog_02 GeoJson [1;91m@db.Geography(GeometryZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointM,4326)` of Postgres: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:52[0m
        [1;94m   | [0m
        [1;94m51 | [0m  geog_02 GeoJson @db.Geography(GeometryZM, 4326)
        [1;94m52 | [0m  geog_03 GeoJson [1;91m@db.Geography(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointZM,4326)` of Postgres: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:53[0m
        [1;94m   | [0m
        [1;94m52 | [0m  geog_03 GeoJson @db.Geography(PointM, 4326)
        [1;94m53 | [0m  geog_04 GeoJson [1;91m@db.Geography(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointM,4326)` of Postgres: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:54[0m
        [1;94m   | [0m
        [1;94m53 | [0m  geog_04 GeoJson @db.Geography(PointZM, 4326)
        [1;94m54 | [0m  geog_05 GeoJson [1;91m@db.Geography(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointZM,4326)` of Postgres: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:55[0m
        [1;94m   | [0m
        [1;94m54 | [0m  geog_05 GeoJson @db.Geography(PointM, 4326)
        [1;94m55 | [0m  geog_06 GeoJson [1;91m@db.Geography(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(LineStringM,4326)` of Postgres: LineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:56[0m
        [1;94m   | [0m
        [1;94m55 | [0m  geog_06 GeoJson @db.Geography(PointZM, 4326)
        [1;94m56 | [0m  geog_07 GeoJson [1;91m@db.Geography(LineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(LineStringZM,4326)` of Postgres: LineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:57[0m
        [1;94m   | [0m
        [1;94m56 | [0m  geog_07 GeoJson @db.Geography(LineStringM, 4326)
        [1;94m57 | [0m  geog_08 GeoJson [1;91m@db.Geography(LineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolygonM,4326)` of Postgres: PolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:58[0m
        [1;94m   | [0m
        [1;94m57 | [0m  geog_08 GeoJson @db.Geography(LineStringZM, 4326)
        [1;94m58 | [0m  geog_09 GeoJson [1;91m@db.Geography(PolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolygonZM,4326)` of Postgres: PolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:59[0m
        [1;94m   | [0m
        [1;94m58 | [0m  geog_09 GeoJson @db.Geography(PolygonM, 4326)
        [1;94m59 | [0m  geog_10 GeoJson [1;91m@db.Geography(PolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPointM,4326)` of Postgres: MultiPointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:60[0m
        [1;94m   | [0m
        [1;94m59 | [0m  geog_10 GeoJson @db.Geography(PolygonZM, 4326)
        [1;94m60 | [0m  geog_11 GeoJson [1;91m@db.Geography(MultiPointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPointZM,4326)` of Postgres: MultiPointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:61[0m
        [1;94m   | [0m
        [1;94m60 | [0m  geog_11 GeoJson @db.Geography(MultiPointM, 4326)
        [1;94m61 | [0m  geog_12 GeoJson [1;91m@db.Geography(MultiPointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiLineStringM,4326)` of Postgres: MultiLineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:62[0m
        [1;94m   | [0m
        [1;94m61 | [0m  geog_12 GeoJson @db.Geography(MultiPointZM, 4326)
        [1;94m62 | [0m  geog_13 GeoJson [1;91m@db.Geography(MultiLineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiLineStringZM,4326)` of Postgres: MultiLineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:63[0m
        [1;94m   | [0m
        [1;94m62 | [0m  geog_13 GeoJson @db.Geography(MultiLineStringM, 4326)
        [1;94m63 | [0m  geog_14 GeoJson [1;91m@db.Geography(MultiLineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPolygonM,4326)` of Postgres: MultiPolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:64[0m
        [1;94m   | [0m
        [1;94m63 | [0m  geog_14 GeoJson @db.Geography(MultiLineStringZM, 4326)
        [1;94m64 | [0m  geog_15 GeoJson [1;91m@db.Geography(MultiPolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPolygonZM,4326)` of Postgres: MultiPolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:65[0m
        [1;94m   | [0m
        [1;94m64 | [0m  geog_15 GeoJson @db.Geography(MultiPolygonM, 4326)
        [1;94m65 | [0m  geog_16 GeoJson [1;91m@db.Geography(MultiPolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryCollectionM,4326)` of Postgres: GeometryCollectionM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:66[0m
        [1;94m   | [0m
        [1;94m65 | [0m  geog_16 GeoJson @db.Geography(MultiPolygonZM, 4326)
        [1;94m66 | [0m  geog_17 GeoJson [1;91m@db.Geography(GeometryCollectionM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryCollectionZM,4326)` of Postgres: GeometryCollectionZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:67[0m
        [1;94m   | [0m
        [1;94m66 | [0m  geog_17 GeoJson @db.Geography(GeometryCollectionM, 4326)
        [1;94m67 | [0m  geog_18 GeoJson [1;91m@db.Geography(GeometryCollectionZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CircularString,4326)` of Postgres: CircularString isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:68[0m
        [1;94m   | [0m
        [1;94m67 | [0m  geog_18 GeoJson @db.Geography(GeometryCollectionZM, 4326)
        [1;94m68 | [0m  geog_19 GeoJson [1;91m@db.Geography(CircularString, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CircularStringZ,4326)` of Postgres: CircularStringZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:69[0m
        [1;94m   | [0m
        [1;94m68 | [0m  geog_19 GeoJson @db.Geography(CircularString, 4326)
        [1;94m69 | [0m  geog_20 GeoJson [1;91m@db.Geography(CircularStringZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CircularStringM,4326)` of Postgres: CircularStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:70[0m
        [1;94m   | [0m
        [1;94m69 | [0m  geog_20 GeoJson @db.Geography(CircularStringZ, 4326)
        [1;94m70 | [0m  geog_21 GeoJson [1;91m@db.Geography(CircularStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CircularStringZM,4326)` of Postgres: CircularStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:71[0m
        [1;94m   | [0m
        [1;94m70 | [0m  geog_21 GeoJson @db.Geography(CircularStringM, 4326)
        [1;94m71 | [0m  geog_22 GeoJson [1;91m@db.Geography(CircularStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CompoundCurve,4326)` of Postgres: CompoundCurve isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:72[0m
        [1;94m   | [0m
        [1;94m71 | [0m  geog_22 GeoJson @db.Geography(CircularStringZM, 4326)
        [1;94m72 | [0m  geog_23 GeoJson [1;91m@db.Geography(CompoundCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CompoundCurveZ,4326)` of Postgres: CompoundCurveZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:73[0m
        [1;94m   | [0m
        [1;94m72 | [0m  geog_23 GeoJson @db.Geography(CompoundCurve, 4326)
        [1;94m73 | [0m  geog_24 GeoJson [1;91m@db.Geography(CompoundCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CompoundCurveM,4326)` of Postgres: CompoundCurveM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:74[0m
        [1;94m   | [0m
        [1;94m73 | [0m  geog_24 GeoJson @db.Geography(CompoundCurveZ, 4326)
        [1;94m74 | [0m  geog_35 GeoJson [1;91m@db.Geography(CompoundCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CompoundCurveZM,4326)` of Postgres: CompoundCurveZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:75[0m
        [1;94m   | [0m
        [1;94m74 | [0m  geog_35 GeoJson @db.Geography(CompoundCurveM, 4326)
        [1;94m75 | [0m  geog_36 GeoJson [1;91m@db.Geography(CompoundCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CurvePolygon,4326)` of Postgres: CurvePolygon isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:76[0m
        [1;94m   | [0m
        [1;94m75 | [0m  geog_36 GeoJson @db.Geography(CompoundCurveZM, 4326)
        [1;94m76 | [0m  geog_37 GeoJson [1;91m@db.Geography(CurvePolygon, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CurvePolygonZ,4326)` of Postgres: CurvePolygonZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:77[0m
        [1;94m   | [0m
        [1;94m76 | [0m  geog_37 GeoJson @db.Geography(CurvePolygon, 4326)
        [1;94m77 | [0m  geog_38 GeoJson [1;91m@db.Geography(CurvePolygonZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CurvePolygonM,4326)` of Postgres: CurvePolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:78[0m
        [1;94m   | [0m
        [1;94m77 | [0m  geog_38 GeoJson @db.Geography(CurvePolygonZ, 4326)
        [1;94m78 | [0m  geog_39 GeoJson [1;91m@db.Geography(CurvePolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(CurvePolygonZM,4326)` of Postgres: CurvePolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:79[0m
        [1;94m   | [0m
        [1;94m78 | [0m  geog_39 GeoJson @db.Geography(CurvePolygonM, 4326)
        [1;94m79 | [0m  geog_40 GeoJson [1;91m@db.Geography(CurvePolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiCurve,4326)` of Postgres: MultiCurve isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:80[0m
        [1;94m   | [0m
        [1;94m79 | [0m  geog_40 GeoJson @db.Geography(CurvePolygonZM, 4326)
        [1;94m80 | [0m  geog_41 GeoJson [1;91m@db.Geography(MultiCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiCurveZ,4326)` of Postgres: MultiCurveZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:81[0m
        [1;94m   | [0m
        [1;94m80 | [0m  geog_41 GeoJson @db.Geography(MultiCurve, 4326)
        [1;94m81 | [0m  geog_42 GeoJson [1;91m@db.Geography(MultiCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiCurveM,4326)` of Postgres: MultiCurveM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:82[0m
        [1;94m   | [0m
        [1;94m81 | [0m  geog_42 GeoJson @db.Geography(MultiCurveZ, 4326)
        [1;94m82 | [0m  geog_43 GeoJson [1;91m@db.Geography(MultiCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiCurveZM,4326)` of Postgres: MultiCurveZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:83[0m
        [1;94m   | [0m
        [1;94m82 | [0m  geog_43 GeoJson @db.Geography(MultiCurveM, 4326)
        [1;94m83 | [0m  geog_44 GeoJson [1;91m@db.Geography(MultiCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiSurface,4326)` of Postgres: MultiSurface isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:84[0m
        [1;94m   | [0m
        [1;94m83 | [0m  geog_44 GeoJson @db.Geography(MultiCurveZM, 4326)
        [1;94m84 | [0m  geog_45 GeoJson [1;91m@db.Geography(MultiSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiSurfaceZ,4326)` of Postgres: MultiSurfaceZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:85[0m
        [1;94m   | [0m
        [1;94m84 | [0m  geog_45 GeoJson @db.Geography(MultiSurface, 4326)
        [1;94m85 | [0m  geog_46 GeoJson [1;91m@db.Geography(MultiSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiSurfaceM,4326)` of Postgres: MultiSurfaceM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:86[0m
        [1;94m   | [0m
        [1;94m85 | [0m  geog_46 GeoJson @db.Geography(MultiSurfaceZ, 4326)
        [1;94m86 | [0m  geog_47 GeoJson [1;91m@db.Geography(MultiSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiSurfaceZM,4326)` of Postgres: MultiSurfaceZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:87[0m
        [1;94m   | [0m
        [1;94m86 | [0m  geog_47 GeoJson @db.Geography(MultiSurfaceM, 4326)
        [1;94m87 | [0m  geog_48 GeoJson [1;91m@db.Geography(MultiSurfaceZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolyhedralSurface,4326)` of Postgres: PolyhedralSurface isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:88[0m
        [1;94m   | [0m
        [1;94m87 | [0m  geog_48 GeoJson @db.Geography(MultiSurfaceZM, 4326)
        [1;94m88 | [0m  geog_49 GeoJson [1;91m@db.Geography(PolyhedralSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolyhedralSurfaceZ,4326)` of Postgres: PolyhedralSurfaceZ isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:89[0m
        [1;94m   | [0m
        [1;94m88 | [0m  geog_49 GeoJson @db.Geography(PolyhedralSurface, 4326)
        [1;94m89 | [0m  geog_50 GeoJson [1;91m@db.Geography(PolyhedralSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolyhedralSurfaceM,4326)` of Postgres: PolyhedralSurfaceM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:90[0m
        [1;94m   | [0m
        [1;94m89 | [0m  geog_50 GeoJson @db.Geography(PolyhedralSurfaceZ, 4326)
        [1;94m90 | [0m  geog_51 GeoJson [1;91m@db.Geography(PolyhedralSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolyhedralSurfaceZM,4326)` of Postgres: PolyhedralSurfaceZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:91[0m
        [1;94m   | [0m
        [1;94m90 | [0m  geog_51 GeoJson @db.Geography(PolyhedralSurfaceM, 4326)
        [1;94m91 | [0m  geog_52 GeoJson [1;91m@db.Geography(PolyhedralSurfaceZM, 4326)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}
