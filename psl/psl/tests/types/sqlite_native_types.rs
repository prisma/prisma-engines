use crate::common::*;
use expect_test::expect;

#[test]
fn sqlite_specific_native_types_are_valid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
        }

        model NativeTypesTest {
          id          Int       @id
          geomcol1    Geometry  @db.Geometry(Geometry, 4326)
          geomcol2    Geometry  @db.Geometry(GeometryZ, 4326)
          geomcol3    Geometry  @db.Geometry(GeometryM, 4326)
          geomcol4    Geometry  @db.Geometry(GeometryZM, 4326)
          geomcol5    Geometry  @db.Geometry(Point, 4326)
          geomcol6    Geometry  @db.Geometry(PointZ, 4326)
          geomcol7    Geometry  @db.Geometry(PointM, 4326)
          geomcol8    Geometry  @db.Geometry(PointZM, 4326)
          geomcol9    Geometry  @db.Geometry(Point, 4326)
          geomcol10   Geometry  @db.Geometry(PointZ, 4326)
          geomcol11   Geometry  @db.Geometry(PointM, 4326)
          geomcol12   Geometry  @db.Geometry(PointZM, 4326)
          geomcol13   Geometry  @db.Geometry(LineString, 4326)
          geomcol14   Geometry  @db.Geometry(LineStringZ, 4326)
          geomcol15   Geometry  @db.Geometry(LineStringM, 4326)
          geomcol16   Geometry  @db.Geometry(LineStringZM, 4326)
          geomcol17   Geometry  @db.Geometry(Polygon, 4326)
          geomcol18   Geometry  @db.Geometry(PolygonZ, 4326)
          geomcol19   Geometry  @db.Geometry(PolygonM, 4326)
          geomcol20   Geometry  @db.Geometry(PolygonZM, 4326)
          geomcol21   Geometry  @db.Geometry(MultiPoint, 4326)
          geomcol22   Geometry  @db.Geometry(MultiPointZ, 4326)
          geomcol23   Geometry  @db.Geometry(MultiPointM, 4326)
          geomcol24   Geometry  @db.Geometry(MultiPointZM, 4326)
          geomcol25   Geometry  @db.Geometry(MultiLineString, 4326)
          geomcol26   Geometry  @db.Geometry(MultiLineStringZ, 4326)
          geomcol27   Geometry  @db.Geometry(MultiLineStringM, 4326)
          geomcol28   Geometry  @db.Geometry(MultiLineStringZM, 4326)
          geomcol29   Geometry  @db.Geometry(MultiPolygon, 4326)
          geomcol30   Geometry  @db.Geometry(MultiPolygonZ, 4326)
          geomcol31   Geometry  @db.Geometry(MultiPolygonM, 4326)
          geomcol32   Geometry  @db.Geometry(MultiPolygonZM, 4326)
          geomcol33   Geometry  @db.Geometry(GeometryCollection, 4326)
          geomcol34   Geometry  @db.Geometry(GeometryCollectionZ, 4326)
          geomcol35   Geometry  @db.Geometry(GeometryCollectionM, 4326)
          geomcol36   Geometry  @db.Geometry(GeometryCollectionZM, 4326)
        }
    "#};

    psl::parse_schema(schema).unwrap();
}

#[test]
fn should_fail_on_geojson_when_invalid_geometry_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
        }

        model Blog {
          id   Int      @id
          geom Geometry @db.Geometry(Invalid)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a geometry type and an optional srid, but found (Invalid).[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id   Int      @id
        [1;94m 8 | [0m  geom Geometry [1;91m@db.Geometry(Invalid)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn should_fail_on_geojson_when_non_wgs84_srid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
        }

        model User {
          id   Int     @id
          geom GeoJson @db.Geometry(Point, 3857)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(Point,3857)` of sqlite: GeoJson SRID must be 4326.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id   Int     @id
        [1;94m 8 | [0m  geom GeoJson [1;91m@db.Geometry(Point, 3857)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_geometry_when_out_of_bound_srid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
        }

        model User {
          id   Int      @id
          geom Geometry @db.Geometry(Point, -2)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(Point,-2)` of sqlite: SRID must be superior or equal to -1.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id   Int      @id
        [1;94m 8 | [0m  geom Geometry [1;91m@db.Geometry(Point, -2)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_geometry_when_extra_geometry_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
        }

        model User {
          id    Int        @id
          geom_00 Geometry @db.Geometry(CircularString, 4326)
          geom_01 Geometry @db.Geometry(CircularStringZ, 4326)
          geom_02 Geometry @db.Geometry(CircularStringM, 4326)
          geom_03 Geometry @db.Geometry(CircularStringZM, 4326)
          geom_04 Geometry @db.Geometry(CompoundCurve, 4326)
          geom_05 Geometry @db.Geometry(CompoundCurveZ, 4326)
          geom_06 Geometry @db.Geometry(CompoundCurveM, 4326)
          geom_07 Geometry @db.Geometry(CompoundCurveZM, 4326)
          geom_08 Geometry @db.Geometry(CurvePolygon, 4326)
          geom_09 Geometry @db.Geometry(CurvePolygonZ, 4326)
          geom_10 Geometry @db.Geometry(CurvePolygonM, 4326)
          geom_11 Geometry @db.Geometry(CurvePolygonZM, 4326)
          geom_12 Geometry @db.Geometry(MultiCurve, 4326)
          geom_13 Geometry @db.Geometry(MultiCurveZ, 4326)
          geom_14 Geometry @db.Geometry(MultiCurveM, 4326)
          geom_15 Geometry @db.Geometry(MultiCurveZM, 4326)
          geom_16 Geometry @db.Geometry(MultiSurface, 4326)
          geom_17 Geometry @db.Geometry(MultiSurfaceZ, 4326)
          geom_18 Geometry @db.Geometry(MultiSurfaceM, 4326)
          geom_19 Geometry @db.Geometry(MultiSurfaceZM, 4326)
          geom_20 Geometry @db.Geometry(PolyhedralSurface, 4326)
          geom_21 Geometry @db.Geometry(PolyhedralSurfaceZ, 4326)
          geom_22 Geometry @db.Geometry(PolyhedralSurfaceM, 4326)
          geom_23 Geometry @db.Geometry(PolyhedralSurfaceZM, 4326)
          geog_00 Geometry @db.Geography(CircularString, 4326)
          geog_01 Geometry @db.Geography(CircularStringZ, 4326)
          geog_02 Geometry @db.Geography(CircularStringM, 4326)
          geog_03 Geometry @db.Geography(CircularStringZM, 4326)
          geog_04 Geometry @db.Geography(CompoundCurve, 4326)
          geog_05 Geometry @db.Geography(CompoundCurveZ, 4326)
          geog_06 Geometry @db.Geography(CompoundCurveM, 4326)
          geog_07 Geometry @db.Geography(CompoundCurveZM, 4326)
          geog_08 Geometry @db.Geography(CurvePolygon, 4326)
          geog_09 Geometry @db.Geography(CurvePolygonZ, 4326)
          geog_10 Geometry @db.Geography(CurvePolygonM, 4326)
          geog_11 Geometry @db.Geography(CurvePolygonZM, 4326)
          geog_12 Geometry @db.Geography(MultiCurve, 4326)
          geog_13 Geometry @db.Geography(MultiCurveZ, 4326)
          geog_14 Geometry @db.Geography(MultiCurveM, 4326)
          geog_15 Geometry @db.Geography(MultiCurveZM, 4326)
          geog_16 Geometry @db.Geography(MultiSurface, 4326)
          geog_17 Geometry @db.Geography(MultiSurfaceZ, 4326)
          geog_18 Geometry @db.Geography(MultiSurfaceM, 4326)
          geog_19 Geometry @db.Geography(MultiSurfaceZM, 4326)
          geog_20 Geometry @db.Geography(PolyhedralSurface, 4326)
          geog_21 Geometry @db.Geography(PolyhedralSurfaceZ, 4326)
          geog_22 Geometry @db.Geography(PolyhedralSurfaceM, 4326)
          geog_23 Geometry @db.Geography(PolyhedralSurfaceZM, 4326)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularString,4326)` of sqlite: CircularString isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id    Int        @id
        [1;94m 8 | [0m  geom_00 Geometry [1;91m@db.Geometry(CircularString, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularStringZ,4326)` of sqlite: CircularStringZ isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  geom_00 Geometry @db.Geometry(CircularString, 4326)
        [1;94m 9 | [0m  geom_01 Geometry [1;91m@db.Geometry(CircularStringZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularStringM,4326)` of sqlite: CircularStringM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  geom_01 Geometry @db.Geometry(CircularStringZ, 4326)
        [1;94m10 | [0m  geom_02 Geometry [1;91m@db.Geometry(CircularStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CircularStringZM,4326)` of sqlite: CircularStringZM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m  geom_02 Geometry @db.Geometry(CircularStringM, 4326)
        [1;94m11 | [0m  geom_03 Geometry [1;91m@db.Geometry(CircularStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurve,4326)` of sqlite: CompoundCurve isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  geom_03 Geometry @db.Geometry(CircularStringZM, 4326)
        [1;94m12 | [0m  geom_04 Geometry [1;91m@db.Geometry(CompoundCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurveZ,4326)` of sqlite: CompoundCurveZ isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  geom_04 Geometry @db.Geometry(CompoundCurve, 4326)
        [1;94m13 | [0m  geom_05 Geometry [1;91m@db.Geometry(CompoundCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurveM,4326)` of sqlite: CompoundCurveM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  geom_05 Geometry @db.Geometry(CompoundCurveZ, 4326)
        [1;94m14 | [0m  geom_06 Geometry [1;91m@db.Geometry(CompoundCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CompoundCurveZM,4326)` of sqlite: CompoundCurveZM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  geom_06 Geometry @db.Geometry(CompoundCurveM, 4326)
        [1;94m15 | [0m  geom_07 Geometry [1;91m@db.Geometry(CompoundCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygon,4326)` of sqlite: CurvePolygon isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  geom_07 Geometry @db.Geometry(CompoundCurveZM, 4326)
        [1;94m16 | [0m  geom_08 Geometry [1;91m@db.Geometry(CurvePolygon, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygonZ,4326)` of sqlite: CurvePolygonZ isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  geom_08 Geometry @db.Geometry(CurvePolygon, 4326)
        [1;94m17 | [0m  geom_09 Geometry [1;91m@db.Geometry(CurvePolygonZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygonM,4326)` of sqlite: CurvePolygonM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  geom_09 Geometry @db.Geometry(CurvePolygonZ, 4326)
        [1;94m18 | [0m  geom_10 Geometry [1;91m@db.Geometry(CurvePolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(CurvePolygonZM,4326)` of sqlite: CurvePolygonZM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m  geom_10 Geometry @db.Geometry(CurvePolygonM, 4326)
        [1;94m19 | [0m  geom_11 Geometry [1;91m@db.Geometry(CurvePolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurve,4326)` of sqlite: MultiCurve isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m  geom_11 Geometry @db.Geometry(CurvePolygonZM, 4326)
        [1;94m20 | [0m  geom_12 Geometry [1;91m@db.Geometry(MultiCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurveZ,4326)` of sqlite: MultiCurveZ isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:21[0m
        [1;94m   | [0m
        [1;94m20 | [0m  geom_12 Geometry @db.Geometry(MultiCurve, 4326)
        [1;94m21 | [0m  geom_13 Geometry [1;91m@db.Geometry(MultiCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurveM,4326)` of sqlite: MultiCurveM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:22[0m
        [1;94m   | [0m
        [1;94m21 | [0m  geom_13 Geometry @db.Geometry(MultiCurveZ, 4326)
        [1;94m22 | [0m  geom_14 Geometry [1;91m@db.Geometry(MultiCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiCurveZM,4326)` of sqlite: MultiCurveZM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:23[0m
        [1;94m   | [0m
        [1;94m22 | [0m  geom_14 Geometry @db.Geometry(MultiCurveM, 4326)
        [1;94m23 | [0m  geom_15 Geometry [1;91m@db.Geometry(MultiCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurface,4326)` of sqlite: MultiSurface isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:24[0m
        [1;94m   | [0m
        [1;94m23 | [0m  geom_15 Geometry @db.Geometry(MultiCurveZM, 4326)
        [1;94m24 | [0m  geom_16 Geometry [1;91m@db.Geometry(MultiSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurfaceZ,4326)` of sqlite: MultiSurfaceZ isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:25[0m
        [1;94m   | [0m
        [1;94m24 | [0m  geom_16 Geometry @db.Geometry(MultiSurface, 4326)
        [1;94m25 | [0m  geom_17 Geometry [1;91m@db.Geometry(MultiSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurfaceM,4326)` of sqlite: MultiSurfaceM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:26[0m
        [1;94m   | [0m
        [1;94m25 | [0m  geom_17 Geometry @db.Geometry(MultiSurfaceZ, 4326)
        [1;94m26 | [0m  geom_18 Geometry [1;91m@db.Geometry(MultiSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiSurfaceZM,4326)` of sqlite: MultiSurfaceZM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:27[0m
        [1;94m   | [0m
        [1;94m26 | [0m  geom_18 Geometry @db.Geometry(MultiSurfaceM, 4326)
        [1;94m27 | [0m  geom_19 Geometry [1;91m@db.Geometry(MultiSurfaceZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurface,4326)` of sqlite: PolyhedralSurface isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:28[0m
        [1;94m   | [0m
        [1;94m27 | [0m  geom_19 Geometry @db.Geometry(MultiSurfaceZM, 4326)
        [1;94m28 | [0m  geom_20 Geometry [1;91m@db.Geometry(PolyhedralSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurfaceZ,4326)` of sqlite: PolyhedralSurfaceZ isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:29[0m
        [1;94m   | [0m
        [1;94m28 | [0m  geom_20 Geometry @db.Geometry(PolyhedralSurface, 4326)
        [1;94m29 | [0m  geom_21 Geometry [1;91m@db.Geometry(PolyhedralSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurfaceM,4326)` of sqlite: PolyhedralSurfaceM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:30[0m
        [1;94m   | [0m
        [1;94m29 | [0m  geom_21 Geometry @db.Geometry(PolyhedralSurfaceZ, 4326)
        [1;94m30 | [0m  geom_22 Geometry [1;91m@db.Geometry(PolyhedralSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolyhedralSurfaceZM,4326)` of sqlite: PolyhedralSurfaceZM isn't supported for the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:31[0m
        [1;94m   | [0m
        [1;94m30 | [0m  geom_22 Geometry @db.Geometry(PolyhedralSurfaceM, 4326)
        [1;94m31 | [0m  geom_23 Geometry [1;91m@db.Geometry(PolyhedralSurfaceZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:32[0m
        [1;94m   | [0m
        [1;94m31 | [0m  geom_23 Geometry @db.Geometry(PolyhedralSurfaceZM, 4326)
        [1;94m32 | [0m  geog_00 Geometry [1;91m@db.Geography(CircularString, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:33[0m
        [1;94m   | [0m
        [1;94m32 | [0m  geog_00 Geometry @db.Geography(CircularString, 4326)
        [1;94m33 | [0m  geog_01 Geometry [1;91m@db.Geography(CircularStringZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:34[0m
        [1;94m   | [0m
        [1;94m33 | [0m  geog_01 Geometry @db.Geography(CircularStringZ, 4326)
        [1;94m34 | [0m  geog_02 Geometry [1;91m@db.Geography(CircularStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:35[0m
        [1;94m   | [0m
        [1;94m34 | [0m  geog_02 Geometry @db.Geography(CircularStringM, 4326)
        [1;94m35 | [0m  geog_03 Geometry [1;91m@db.Geography(CircularStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:36[0m
        [1;94m   | [0m
        [1;94m35 | [0m  geog_03 Geometry @db.Geography(CircularStringZM, 4326)
        [1;94m36 | [0m  geog_04 Geometry [1;91m@db.Geography(CompoundCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:37[0m
        [1;94m   | [0m
        [1;94m36 | [0m  geog_04 Geometry @db.Geography(CompoundCurve, 4326)
        [1;94m37 | [0m  geog_05 Geometry [1;91m@db.Geography(CompoundCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:38[0m
        [1;94m   | [0m
        [1;94m37 | [0m  geog_05 Geometry @db.Geography(CompoundCurveZ, 4326)
        [1;94m38 | [0m  geog_06 Geometry [1;91m@db.Geography(CompoundCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:39[0m
        [1;94m   | [0m
        [1;94m38 | [0m  geog_06 Geometry @db.Geography(CompoundCurveM, 4326)
        [1;94m39 | [0m  geog_07 Geometry [1;91m@db.Geography(CompoundCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:40[0m
        [1;94m   | [0m
        [1;94m39 | [0m  geog_07 Geometry @db.Geography(CompoundCurveZM, 4326)
        [1;94m40 | [0m  geog_08 Geometry [1;91m@db.Geography(CurvePolygon, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:41[0m
        [1;94m   | [0m
        [1;94m40 | [0m  geog_08 Geometry @db.Geography(CurvePolygon, 4326)
        [1;94m41 | [0m  geog_09 Geometry [1;91m@db.Geography(CurvePolygonZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:42[0m
        [1;94m   | [0m
        [1;94m41 | [0m  geog_09 Geometry @db.Geography(CurvePolygonZ, 4326)
        [1;94m42 | [0m  geog_10 Geometry [1;91m@db.Geography(CurvePolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:43[0m
        [1;94m   | [0m
        [1;94m42 | [0m  geog_10 Geometry @db.Geography(CurvePolygonM, 4326)
        [1;94m43 | [0m  geog_11 Geometry [1;91m@db.Geography(CurvePolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:44[0m
        [1;94m   | [0m
        [1;94m43 | [0m  geog_11 Geometry @db.Geography(CurvePolygonZM, 4326)
        [1;94m44 | [0m  geog_12 Geometry [1;91m@db.Geography(MultiCurve, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:45[0m
        [1;94m   | [0m
        [1;94m44 | [0m  geog_12 Geometry @db.Geography(MultiCurve, 4326)
        [1;94m45 | [0m  geog_13 Geometry [1;91m@db.Geography(MultiCurveZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:46[0m
        [1;94m   | [0m
        [1;94m45 | [0m  geog_13 Geometry @db.Geography(MultiCurveZ, 4326)
        [1;94m46 | [0m  geog_14 Geometry [1;91m@db.Geography(MultiCurveM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:47[0m
        [1;94m   | [0m
        [1;94m46 | [0m  geog_14 Geometry @db.Geography(MultiCurveM, 4326)
        [1;94m47 | [0m  geog_15 Geometry [1;91m@db.Geography(MultiCurveZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:48[0m
        [1;94m   | [0m
        [1;94m47 | [0m  geog_15 Geometry @db.Geography(MultiCurveZM, 4326)
        [1;94m48 | [0m  geog_16 Geometry [1;91m@db.Geography(MultiSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:49[0m
        [1;94m   | [0m
        [1;94m48 | [0m  geog_16 Geometry @db.Geography(MultiSurface, 4326)
        [1;94m49 | [0m  geog_17 Geometry [1;91m@db.Geography(MultiSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:50[0m
        [1;94m   | [0m
        [1;94m49 | [0m  geog_17 Geometry @db.Geography(MultiSurfaceZ, 4326)
        [1;94m50 | [0m  geog_18 Geometry [1;91m@db.Geography(MultiSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:51[0m
        [1;94m   | [0m
        [1;94m50 | [0m  geog_18 Geometry @db.Geography(MultiSurfaceM, 4326)
        [1;94m51 | [0m  geog_19 Geometry [1;91m@db.Geography(MultiSurfaceZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:52[0m
        [1;94m   | [0m
        [1;94m51 | [0m  geog_19 Geometry @db.Geography(MultiSurfaceZM, 4326)
        [1;94m52 | [0m  geog_20 Geometry [1;91m@db.Geography(PolyhedralSurface, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:53[0m
        [1;94m   | [0m
        [1;94m52 | [0m  geog_20 Geometry @db.Geography(PolyhedralSurface, 4326)
        [1;94m53 | [0m  geog_21 Geometry [1;91m@db.Geography(PolyhedralSurfaceZ, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:54[0m
        [1;94m   | [0m
        [1;94m53 | [0m  geog_21 Geometry @db.Geography(PolyhedralSurfaceZ, 4326)
        [1;94m54 | [0m  geog_22 Geometry [1;91m@db.Geography(PolyhedralSurfaceM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:55[0m
        [1;94m   | [0m
        [1;94m54 | [0m  geog_22 Geometry @db.Geography(PolyhedralSurfaceM, 4326)
        [1;94m55 | [0m  geog_23 Geometry [1;91m@db.Geography(PolyhedralSurfaceZM, 4326)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}
