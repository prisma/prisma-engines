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
          id        Int      @id
          geomcol1  Geometry @db.Geometry(Geometry, 4326)
          geomcol2  Geometry @db.Geometry(GeometryZ, 4326)
          geomcol3  Geometry @db.Geometry(GeometryM, 4326)
          geomcol4  Geometry @db.Geometry(GeometryZM, 4326)
          geomcol5  Geometry @db.Geometry(Point, 4326)
          geomcol6  Geometry @db.Geometry(PointZ, 4326)
          geomcol7  Geometry @db.Geometry(PointM, 4326)
          geomcol8  Geometry @db.Geometry(PointZM, 4326)
          geomcol9  Geometry @db.Geometry(Point, 4326)
          geomcol10 Geometry @db.Geometry(PointZ, 4326)
          geomcol11 Geometry @db.Geometry(PointM, 4326)
          geomcol12 Geometry @db.Geometry(PointZM, 4326)
          geomcol13 Geometry @db.Geometry(LineString, 4326)
          geomcol14 Geometry @db.Geometry(LineStringZ, 4326)
          geomcol15 Geometry @db.Geometry(LineStringM, 4326)
          geomcol16 Geometry @db.Geometry(LineStringZM, 4326)
          geomcol17 Geometry @db.Geometry(Polygon, 4326)
          geomcol18 Geometry @db.Geometry(PolygonZ, 4326)
          geomcol19 Geometry @db.Geometry(PolygonM, 4326)
          geomcol20 Geometry @db.Geometry(PolygonZM, 4326)
          geomcol21 Geometry @db.Geometry(MultiPoint, 4326)
          geomcol22 Geometry @db.Geometry(MultiPointZ, 4326)
          geomcol23 Geometry @db.Geometry(MultiPointM, 4326)
          geomcol24 Geometry @db.Geometry(MultiPointZM, 4326)
          geomcol25 Geometry @db.Geometry(MultiLineString, 4326)
          geomcol26 Geometry @db.Geometry(MultiLineStringZ, 4326)
          geomcol27 Geometry @db.Geometry(MultiLineStringM, 4326)
          geomcol28 Geometry @db.Geometry(MultiLineStringZM, 4326)
          geomcol29 Geometry @db.Geometry(MultiPolygon, 4326)
          geomcol30 Geometry @db.Geometry(MultiPolygonZ, 4326)
          geomcol31 Geometry @db.Geometry(MultiPolygonM, 4326)
          geomcol32 Geometry @db.Geometry(MultiPolygonZM, 4326)
          geomcol33 Geometry @db.Geometry(GeometryCollection, 4326)
          geomcol34 Geometry @db.Geometry(GeometryCollectionZ, 4326)
          geomcol35 Geometry @db.Geometry(GeometryCollectionM, 4326)
          geomcol36 Geometry @db.Geometry(GeometryCollectionZM, 4326)
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
fn should_fail_on_geography() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
        }

        model User {
          id  Int       @id
          geog Geometry @db.Geography
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type Geography is not supported for sqlite connector.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int       @id
        [1;94m 8 | [0m  geog Geometry [1;91m@db.Geography[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}
