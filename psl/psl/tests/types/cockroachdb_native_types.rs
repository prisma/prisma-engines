use crate::common::*;
use expect_test::expect;

#[test]
fn should_fail_on_invalid_precision_for_decimal_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int     @id
          firstName Decimal @db.Decimal(1001,3)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(1001,3)` of CockroachDB: Precision must be positive with a maximum value of 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int     @id
        [1;94m 8 | [0m  firstName Decimal [1;91m@db.Decimal(1001,3)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_invalid_precision_for_time_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Time(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Time(7)` of CockroachDB: M can range from 0 to 6.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int      @id
        [1;94m 8 | [0m  val DateTime [1;91m@db.Time(7)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Timestamp(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Timestamp(7)` of CockroachDB: M can range from 0 to 6.[0m
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
          provider = "cockroachdb"
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
          provider = "cockroachdb"
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
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int     @id
          dec Decimal @db.Decimal(2, 4)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in CockroachDB.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int     @id
        [1;94m 8 | [0m  dec Decimal [1;91m@db.Decimal(2, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn cockroach_specific_native_types_are_valid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url = env("TEST_DATABASE_URL")
        }

        model NativeTypesTest {
          id          Int      @db.Int4 @id @default(sequence())
          bitcol      String   @db.Bit
          boolcol     Boolean  @db.Bool
          bytescol    Bytes    @db.Bytes
          charcol     String   @db.Char(5)
          datecol     DateTime @db.Date
          decimalcol  Decimal  @db.Decimal(5, 2)
          float4col   Float    @db.Float4
          float8col   Float    @db.Float8
          inetcol     String   @db.Inet
          int2col     Int      @db.Int2
          int4col     Int      @db.Int4
          int8col     BigInt   @db.Int8
          jsonbcol    Json     @db.JsonB
          oidcol      Int      @db.Oid
          scharcol    String   @db.CatalogSingleChar
          stringcol1  String   @db.String
          stringcol2  String   @db.String(40)
          timecol     DateTime @db.Time
          timetzcol   DateTime @db.Timetz
          timestcol   DateTime @db.Timestamp
          timesttzcol DateTime @db.Timestamptz
          uuidcol     String   @db.Uuid
          varbitcol   String   @db.VarBit(200)
          geomcol1    Geometry @db.Geometry(Geometry, 4326)
          geomcol2    Geometry @db.Geometry(GeometryZ, 4326)
          geomcol3    Geometry @db.Geometry(GeometryM, 4326)
          geomcol4    Geometry @db.Geometry(GeometryZM, 4326)
          geomcol5    Geometry @db.Geometry(Point, 4326)
          geomcol6    Geometry @db.Geometry(PointZ, 4326)
          geomcol7    Geometry @db.Geometry(PointM, 4326)
          geomcol8    Geometry @db.Geometry(PointZM, 4326)
          geomcol9    Geometry @db.Geometry(Point, 4326)
          geomcol10   Geometry @db.Geometry(PointZ, 4326)
          geomcol11   Geometry @db.Geometry(PointM, 4326)
          geomcol12   Geometry @db.Geometry(PointZM, 4326)
          geomcol13   Geometry @db.Geometry(LineString, 4326)
          geomcol14   Geometry @db.Geometry(LineStringZ, 4326)
          geomcol15   Geometry @db.Geometry(LineStringM, 4326)
          geomcol16   Geometry @db.Geometry(LineStringZM, 4326)
          geomcol17   Geometry @db.Geometry(Polygon, 4326)
          geomcol18   Geometry @db.Geometry(PolygonZ, 4326)
          geomcol19   Geometry @db.Geometry(PolygonM, 4326)
          geomcol20   Geometry @db.Geometry(PolygonZM, 4326)
          geomcol21   Geometry @db.Geometry(MultiPoint, 4326)
          geomcol22   Geometry @db.Geometry(MultiPointZ, 4326)
          geomcol23   Geometry @db.Geometry(MultiPointM, 4326)
          geomcol24   Geometry @db.Geometry(MultiPointZM, 4326)
          geomcol25   Geometry @db.Geometry(MultiLineString, 4326)
          geomcol26   Geometry @db.Geometry(MultiLineStringZ, 4326)
          geomcol27   Geometry @db.Geometry(MultiLineStringM, 4326)
          geomcol28   Geometry @db.Geometry(MultiLineStringZM, 4326)
          geomcol29   Geometry @db.Geometry(MultiPolygon, 4326)
          geomcol30   Geometry @db.Geometry(MultiPolygonZ, 4326)
          geomcol31   Geometry @db.Geometry(MultiPolygonM, 4326)
          geomcol32   Geometry @db.Geometry(MultiPolygonZM, 4326)
          geomcol33   Geometry @db.Geometry(GeometryCollection, 4326)
          geomcol34   Geometry @db.Geometry(GeometryCollectionZ, 4326)
          geomcol35   Geometry @db.Geometry(GeometryCollectionM, 4326)
          geomcol36   Geometry @db.Geometry(GeometryCollectionZM, 4326)
          geogcol1    Geometry @db.Geography(Geometry, 4326)
          geogcol2    Geometry @db.Geography(GeometryZ, 4326)
          geogcol3    Geometry @db.Geography(GeometryM, 4326)
          geogcol4    Geometry @db.Geography(GeometryZM, 4326)
          geogcol5    Geometry @db.Geography(Point, 4326)
          geogcol6    Geometry @db.Geography(PointZ, 4326)
          geogcol7    Geometry @db.Geography(PointM, 4326)
          geogcol8    Geometry @db.Geography(PointZM, 4326)
          geogcol9    Geometry @db.Geography(Point, 4326)
          geogcol10   Geometry @db.Geography(PointZ, 4326)
          geogcol11   Geometry @db.Geography(PointM, 4326)
          geogcol12   Geometry @db.Geography(PointZM, 4326)
          geogcol13   Geometry @db.Geography(LineString, 4326)
          geogcol14   Geometry @db.Geography(LineStringZ, 4326)
          geogcol15   Geometry @db.Geography(LineStringM, 4326)
          geogcol16   Geometry @db.Geography(LineStringZM, 4326)
          geogcol17   Geometry @db.Geography(Polygon, 4326)
          geogcol18   Geometry @db.Geography(PolygonZ, 4326)
          geogcol19   Geometry @db.Geography(PolygonM, 4326)
          geogcol20   Geometry @db.Geography(PolygonZM, 4326)
          geogcol21   Geometry @db.Geography(MultiPoint, 4326)
          geogcol22   Geometry @db.Geography(MultiPointZ, 4326)
          geogcol23   Geometry @db.Geography(MultiPointM, 4326)
          geogcol24   Geometry @db.Geography(MultiPointZM, 4326)
          geogcol25   Geometry @db.Geography(MultiLineString, 4326)
          geogcol26   Geometry @db.Geography(MultiLineStringZ, 4326)
          geogcol27   Geometry @db.Geography(MultiLineStringM, 4326)
          geogcol28   Geometry @db.Geography(MultiLineStringZM, 4326)
          geogcol29   Geometry @db.Geography(MultiPolygon, 4326)
          geogcol30   Geometry @db.Geography(MultiPolygonZ, 4326)
          geogcol31   Geometry @db.Geography(MultiPolygonM, 4326)
          geogcol32   Geometry @db.Geography(MultiPolygonZM, 4326)
          geogcol33   Geometry @db.Geography(GeometryCollection, 4326)
          geogcol34   Geometry @db.Geography(GeometryCollectionZ, 4326)
          geogcol35   Geometry @db.Geography(GeometryCollectionM, 4326)
          geogcol36   Geometry @db.Geography(GeometryCollectionZM, 4326)
        }
    "#};

    psl::parse_schema(schema).unwrap();
}

#[test]
fn should_fail_on_geojson_when_incompatible_geometry_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id        Int     @id
          geomcol1  GeoJson @db.Geometry(GeometryM, 4326)
          geomcol2  GeoJson @db.Geometry(GeometryZM, 4326)
          geomcol3  GeoJson @db.Geometry(PointM, 4326)
          geomcol4  GeoJson @db.Geometry(PointZM, 4326)
          geomcol5  GeoJson @db.Geometry(PointM, 4326)
          geomcol6  GeoJson @db.Geometry(PointZM, 4326)
          geomcol7  GeoJson @db.Geometry(LineStringM, 4326)
          geomcol8  GeoJson @db.Geometry(LineStringZM, 4326)
          geomcol9  GeoJson @db.Geometry(PolygonM, 4326)
          geomcol10 GeoJson @db.Geometry(PolygonZM, 4326)
          geomcol11 GeoJson @db.Geometry(MultiPointM, 4326)
          geomcol12 GeoJson @db.Geometry(MultiPointZM, 4326)
          geomcol13 GeoJson @db.Geometry(MultiLineStringM, 4326)
          geomcol14 GeoJson @db.Geometry(MultiLineStringZM, 4326)
          geomcol15 GeoJson @db.Geometry(MultiPolygonM, 4326)
          geomcol16 GeoJson @db.Geometry(MultiPolygonZM, 4326)
          geomcol17 GeoJson @db.Geometry(GeometryCollectionM, 4326)
          geomcol18 GeoJson @db.Geometry(GeometryCollectionZM, 4326)
          geogcol1  GeoJson @db.Geography(GeometryM, 4326)
          geogcol2  GeoJson @db.Geography(GeometryZM, 4326)
          geogcol3  GeoJson @db.Geography(PointM, 4326)
          geogcol4  GeoJson @db.Geography(PointZM, 4326)
          geogcol5  GeoJson @db.Geography(PointM, 4326)
          geogcol6  GeoJson @db.Geography(PointZM, 4326)
          geogcol7  GeoJson @db.Geography(LineStringM, 4326)
          geogcol8  GeoJson @db.Geography(LineStringZM, 4326)
          geogcol9  GeoJson @db.Geography(PolygonM, 4326)
          geogcol10 GeoJson @db.Geography(PolygonZM, 4326)
          geogcol11 GeoJson @db.Geography(MultiPointM, 4326)
          geogcol12 GeoJson @db.Geography(MultiPointZM, 4326)
          geogcol13 GeoJson @db.Geography(MultiLineStringM, 4326)
          geogcol14 GeoJson @db.Geography(MultiLineStringZM, 4326)
          geogcol15 GeoJson @db.Geography(MultiPolygonM, 4326)
          geogcol16 GeoJson @db.Geography(MultiPolygonZM, 4326)
          geogcol17 GeoJson @db.Geography(GeometryCollectionM, 4326)
          geogcol18 GeoJson @db.Geography(GeometryCollectionZM, 4326)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryM,4326)` of CockroachDB: GeometryM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int     @id
        [1;94m 8 | [0m  geomcol1  GeoJson [1;91m@db.Geometry(GeometryM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryZM,4326)` of CockroachDB: GeometryZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  geomcol1  GeoJson @db.Geometry(GeometryM, 4326)
        [1;94m 9 | [0m  geomcol2  GeoJson [1;91m@db.Geometry(GeometryZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointM,4326)` of CockroachDB: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  geomcol2  GeoJson @db.Geometry(GeometryZM, 4326)
        [1;94m10 | [0m  geomcol3  GeoJson [1;91m@db.Geometry(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointZM,4326)` of CockroachDB: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m  geomcol3  GeoJson @db.Geometry(PointM, 4326)
        [1;94m11 | [0m  geomcol4  GeoJson [1;91m@db.Geometry(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointM,4326)` of CockroachDB: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  geomcol4  GeoJson @db.Geometry(PointZM, 4326)
        [1;94m12 | [0m  geomcol5  GeoJson [1;91m@db.Geometry(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PointZM,4326)` of CockroachDB: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  geomcol5  GeoJson @db.Geometry(PointM, 4326)
        [1;94m13 | [0m  geomcol6  GeoJson [1;91m@db.Geometry(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(LineStringM,4326)` of CockroachDB: LineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  geomcol6  GeoJson @db.Geometry(PointZM, 4326)
        [1;94m14 | [0m  geomcol7  GeoJson [1;91m@db.Geometry(LineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(LineStringZM,4326)` of CockroachDB: LineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m  geomcol7  GeoJson @db.Geometry(LineStringM, 4326)
        [1;94m15 | [0m  geomcol8  GeoJson [1;91m@db.Geometry(LineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolygonM,4326)` of CockroachDB: PolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  geomcol8  GeoJson @db.Geometry(LineStringZM, 4326)
        [1;94m16 | [0m  geomcol9  GeoJson [1;91m@db.Geometry(PolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(PolygonZM,4326)` of CockroachDB: PolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  geomcol9  GeoJson @db.Geometry(PolygonM, 4326)
        [1;94m17 | [0m  geomcol10 GeoJson [1;91m@db.Geometry(PolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPointM,4326)` of CockroachDB: MultiPointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  geomcol10 GeoJson @db.Geometry(PolygonZM, 4326)
        [1;94m18 | [0m  geomcol11 GeoJson [1;91m@db.Geometry(MultiPointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPointZM,4326)` of CockroachDB: MultiPointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m  geomcol11 GeoJson @db.Geometry(MultiPointM, 4326)
        [1;94m19 | [0m  geomcol12 GeoJson [1;91m@db.Geometry(MultiPointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiLineStringM,4326)` of CockroachDB: MultiLineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m  geomcol12 GeoJson @db.Geometry(MultiPointZM, 4326)
        [1;94m20 | [0m  geomcol13 GeoJson [1;91m@db.Geometry(MultiLineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiLineStringZM,4326)` of CockroachDB: MultiLineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:21[0m
        [1;94m   | [0m
        [1;94m20 | [0m  geomcol13 GeoJson @db.Geometry(MultiLineStringM, 4326)
        [1;94m21 | [0m  geomcol14 GeoJson [1;91m@db.Geometry(MultiLineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPolygonM,4326)` of CockroachDB: MultiPolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:22[0m
        [1;94m   | [0m
        [1;94m21 | [0m  geomcol14 GeoJson @db.Geometry(MultiLineStringZM, 4326)
        [1;94m22 | [0m  geomcol15 GeoJson [1;91m@db.Geometry(MultiPolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(MultiPolygonZM,4326)` of CockroachDB: MultiPolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:23[0m
        [1;94m   | [0m
        [1;94m22 | [0m  geomcol15 GeoJson @db.Geometry(MultiPolygonM, 4326)
        [1;94m23 | [0m  geomcol16 GeoJson [1;91m@db.Geometry(MultiPolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryCollectionM,4326)` of CockroachDB: GeometryCollectionM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:24[0m
        [1;94m   | [0m
        [1;94m23 | [0m  geomcol16 GeoJson @db.Geometry(MultiPolygonZM, 4326)
        [1;94m24 | [0m  geomcol17 GeoJson [1;91m@db.Geometry(GeometryCollectionM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(GeometryCollectionZM,4326)` of CockroachDB: GeometryCollectionZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:25[0m
        [1;94m   | [0m
        [1;94m24 | [0m  geomcol17 GeoJson @db.Geometry(GeometryCollectionM, 4326)
        [1;94m25 | [0m  geomcol18 GeoJson [1;91m@db.Geometry(GeometryCollectionZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryM,4326)` of CockroachDB: GeometryM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:26[0m
        [1;94m   | [0m
        [1;94m25 | [0m  geomcol18 GeoJson @db.Geometry(GeometryCollectionZM, 4326)
        [1;94m26 | [0m  geogcol1  GeoJson [1;91m@db.Geography(GeometryM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryZM,4326)` of CockroachDB: GeometryZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:27[0m
        [1;94m   | [0m
        [1;94m26 | [0m  geogcol1  GeoJson @db.Geography(GeometryM, 4326)
        [1;94m27 | [0m  geogcol2  GeoJson [1;91m@db.Geography(GeometryZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointM,4326)` of CockroachDB: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:28[0m
        [1;94m   | [0m
        [1;94m27 | [0m  geogcol2  GeoJson @db.Geography(GeometryZM, 4326)
        [1;94m28 | [0m  geogcol3  GeoJson [1;91m@db.Geography(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointZM,4326)` of CockroachDB: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:29[0m
        [1;94m   | [0m
        [1;94m28 | [0m  geogcol3  GeoJson @db.Geography(PointM, 4326)
        [1;94m29 | [0m  geogcol4  GeoJson [1;91m@db.Geography(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointM,4326)` of CockroachDB: PointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:30[0m
        [1;94m   | [0m
        [1;94m29 | [0m  geogcol4  GeoJson @db.Geography(PointZM, 4326)
        [1;94m30 | [0m  geogcol5  GeoJson [1;91m@db.Geography(PointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PointZM,4326)` of CockroachDB: PointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:31[0m
        [1;94m   | [0m
        [1;94m30 | [0m  geogcol5  GeoJson @db.Geography(PointM, 4326)
        [1;94m31 | [0m  geogcol6  GeoJson [1;91m@db.Geography(PointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(LineStringM,4326)` of CockroachDB: LineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:32[0m
        [1;94m   | [0m
        [1;94m31 | [0m  geogcol6  GeoJson @db.Geography(PointZM, 4326)
        [1;94m32 | [0m  geogcol7  GeoJson [1;91m@db.Geography(LineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(LineStringZM,4326)` of CockroachDB: LineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:33[0m
        [1;94m   | [0m
        [1;94m32 | [0m  geogcol7  GeoJson @db.Geography(LineStringM, 4326)
        [1;94m33 | [0m  geogcol8  GeoJson [1;91m@db.Geography(LineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolygonM,4326)` of CockroachDB: PolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:34[0m
        [1;94m   | [0m
        [1;94m33 | [0m  geogcol8  GeoJson @db.Geography(LineStringZM, 4326)
        [1;94m34 | [0m  geogcol9  GeoJson [1;91m@db.Geography(PolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(PolygonZM,4326)` of CockroachDB: PolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:35[0m
        [1;94m   | [0m
        [1;94m34 | [0m  geogcol9  GeoJson @db.Geography(PolygonM, 4326)
        [1;94m35 | [0m  geogcol10 GeoJson [1;91m@db.Geography(PolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPointM,4326)` of CockroachDB: MultiPointM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:36[0m
        [1;94m   | [0m
        [1;94m35 | [0m  geogcol10 GeoJson @db.Geography(PolygonZM, 4326)
        [1;94m36 | [0m  geogcol11 GeoJson [1;91m@db.Geography(MultiPointM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPointZM,4326)` of CockroachDB: MultiPointZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:37[0m
        [1;94m   | [0m
        [1;94m36 | [0m  geogcol11 GeoJson @db.Geography(MultiPointM, 4326)
        [1;94m37 | [0m  geogcol12 GeoJson [1;91m@db.Geography(MultiPointZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiLineStringM,4326)` of CockroachDB: MultiLineStringM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:38[0m
        [1;94m   | [0m
        [1;94m37 | [0m  geogcol12 GeoJson @db.Geography(MultiPointZM, 4326)
        [1;94m38 | [0m  geogcol13 GeoJson [1;91m@db.Geography(MultiLineStringM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiLineStringZM,4326)` of CockroachDB: MultiLineStringZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:39[0m
        [1;94m   | [0m
        [1;94m38 | [0m  geogcol13 GeoJson @db.Geography(MultiLineStringM, 4326)
        [1;94m39 | [0m  geogcol14 GeoJson [1;91m@db.Geography(MultiLineStringZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPolygonM,4326)` of CockroachDB: MultiPolygonM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:40[0m
        [1;94m   | [0m
        [1;94m39 | [0m  geogcol14 GeoJson @db.Geography(MultiLineStringZM, 4326)
        [1;94m40 | [0m  geogcol15 GeoJson [1;91m@db.Geography(MultiPolygonM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(MultiPolygonZM,4326)` of CockroachDB: MultiPolygonZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:41[0m
        [1;94m   | [0m
        [1;94m40 | [0m  geogcol15 GeoJson @db.Geography(MultiPolygonM, 4326)
        [1;94m41 | [0m  geogcol16 GeoJson [1;91m@db.Geography(MultiPolygonZM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryCollectionM,4326)` of CockroachDB: GeometryCollectionM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:42[0m
        [1;94m   | [0m
        [1;94m41 | [0m  geogcol16 GeoJson @db.Geography(MultiPolygonZM, 4326)
        [1;94m42 | [0m  geogcol17 GeoJson [1;91m@db.Geography(GeometryCollectionM, 4326)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geography(GeometryCollectionZM,4326)` of CockroachDB: GeometryCollectionZM isn't compatible with GeoJson.[0m
          [1;94m-->[0m  [4mschema.prisma:43[0m
        [1;94m   | [0m
        [1;94m42 | [0m  geogcol17 GeoJson @db.Geography(GeometryCollectionM, 4326)
        [1;94m43 | [0m  geogcol18 GeoJson [1;91m@db.Geography(GeometryCollectionZM, 4326)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn should_fail_on_geojson_when_invalid_geometry_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id   Int     @id
          geom GeoJson @db.Geometry(Invalid)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a geometry type and an optional srid, but found (Invalid).[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id   Int     @id
        [1;94m 8 | [0m  geom GeoJson [1;91m@db.Geometry(Invalid)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn should_fail_on_geojson_when_non_wgs84_srid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id   Int     @id
          geom GeoJson @db.Geometry(Point, 3857)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(Point,3857)` of CockroachDB: GeoJson SRID must be 4326.[0m
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
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id    Int      @id
          geom1 Geometry @db.Geometry(Point, -1)
          geom2 Geometry @db.Geometry(Point, 1000000)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(Point,-1)` of CockroachDB: SRID must be between 0 and 999000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id    Int      @id
        [1;94m 8 | [0m  geom1 Geometry [1;91m@db.Geometry(Point, -1)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `Geometry(Point,1000000)` of CockroachDB: SRID must be between 0 and 999000.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  geom1 Geometry @db.Geometry(Point, -1)
        [1;94m 9 | [0m  geom2 Geometry [1;91m@db.Geometry(Point, 1000000)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}
