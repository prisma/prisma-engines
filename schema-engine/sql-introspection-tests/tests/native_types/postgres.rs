use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

const TYPES: &[(&str, &str)] = &[
    //fieldname, db datatype
    ("smallint", "SmallInt"),
    ("int", "Integer"),
    ("bigint", "BigInt"),
    ("decimal", "Decimal(4, 2)"),
    ("numeric", "Numeric(4, 2)"),
    ("real", "Real"),
    ("doublePrecision", "Double Precision"),
    ("smallSerial", "SmallSerial"),
    ("serial", "Serial"),
    ("bigSerial", "BigSerial"),
    ("varChar", "VarChar(200)"),
    ("char", "Char(200)"),
    ("text", "Text"),
    ("bytea", "ByteA"),
    ("ts", "Timestamp(0)"),
    ("tstz", "Timestamptz(2)"),
    ("date", "Date"),
    ("time", "Time(2)"),
    ("time_2", "Time"),
    ("timetz", "Timetz(2)"),
    ("bool", "Boolean"),
    ("bit", "Bit(1)"),
    ("varbit", "VarBit(1)"),
    ("uuid", "Uuid"),
    ("xml", "Xml"),
    ("json", "Json"),
    ("jsonb", "JsonB"),
    ("money", "Money"),
    ("oid", "Oid"),
    ("inet", "Inet"),
];

const GEOMETRY_TYPES: &[(&str, &str)] = &[
    ("geometry", "Geometry"),
    ("geometry_srid", "Geometry(Geometry, 3857)"),
    ("geometry_geometry_m", "Geometry(GeometryM)"),
    ("geometry_geometry_z", "Geometry(GeometryZ)"),
    ("geometry_geometry_zm", "Geometry(GeometryZM)"),
    ("geometry_point", "Geometry(Point)"),
    ("geometry_point_m", "Geometry(PointM)"),
    ("geometry_point_z", "Geometry(PointZ)"),
    ("geometry_point_zm", "Geometry(PointZM)"),
    ("geometry_linestring", "Geometry(LineString)"),
    ("geometry_linestring_m", "Geometry(LineStringM)"),
    ("geometry_linestring_z", "Geometry(LineStringZ)"),
    ("geometry_linestring_zm", "Geometry(LineStringZM)"),
    ("geometry_polygon", "Geometry(Polygon)"),
    ("geometry_polygon_m", "Geometry(PolygonM)"),
    ("geometry_polygon_z", "Geometry(PolygonZ)"),
    ("geometry_polygon_zm", "Geometry(PolygonZM)"),
    ("geometry_multipoint", "Geometry(MultiPoint)"),
    ("geometry_multipoint_m", "Geometry(MultiPointM)"),
    ("geometry_multipoint_z", "Geometry(MultiPointZ)"),
    ("geometry_multipoint_zm", "Geometry(MultiPointZM)"),
    ("geometry_multilinestring", "Geometry(MultiLineString)"),
    ("geometry_multilinestring_m", "Geometry(MultiLineStringM)"),
    ("geometry_multilinestring_z", "Geometry(MultiLineStringZ)"),
    ("geometry_multilinestring_zm", "Geometry(MultiLineStringZM)"),
    ("geometry_multipolygon", "Geometry(MultiPolygon)"),
    ("geometry_multipolygon_m", "Geometry(MultiPolygonM)"),
    ("geometry_multipolygon_z", "Geometry(MultiPolygonZ)"),
    ("geometry_multipolygon_zm", "Geometry(MultiPolygonZM)"),
    ("geometry_geometrycollection", "Geometry(GeometryCollection)"),
    ("geometry_geometrycollection_m", "Geometry(GeometryCollectionM)"),
    ("geometry_geometrycollection_z", "Geometry(GeometryCollectionZ)"),
    ("geometry_geometrycollection_zm", "Geometry(GeometryCollectionZM)"),
    ("geography_geometry", "Geography(Geometry, 4326)"),
    ("geography_geometry_m", "Geography(GeometryM, 4326)"),
    ("geography_geometry_z", "Geography(GeometryZ, 4326)"),
    ("geography_geometry_zm", "Geography(GeometryZM, 4326)"),
    ("geography_point", "Geography(Point, 4326)"),
    ("geography_point_m", "Geography(PointM, 4326)"),
    ("geography_point_z", "Geography(PointZ, 4326)"),
    ("geography_point_zm", "Geography(PointZM, 4326)"),
    ("geography_linestring", "Geography(LineString, 4326)"),
    ("geography_linestring_m", "Geography(LineStringM, 4326)"),
    ("geography_linestring_z", "Geography(LineStringZ, 4326)"),
    ("geography_linestring_zm", "Geography(LineStringZM, 4326)"),
    ("geography_polygon", "Geography(Polygon, 4326)"),
    ("geography_polygon_m", "Geography(PolygonM, 4326)"),
    ("geography_polygon_z", "Geography(PolygonZ, 4326)"),
    ("geography_polygon_zm", "Geography(PolygonZM, 4326)"),
    ("geography_multipoint", "Geography(MultiPoint, 4326)"),
    ("geography_multipoint_m", "Geography(MultiPointM, 4326)"),
    ("geography_multipoint_z", "Geography(MultiPointZ, 4326)"),
    ("geography_multipoint_zm", "Geography(MultiPointZM, 4326)"),
    ("geography_multilinestring", "Geography(MultiLineString, 4326)"),
    ("geography_multilinestring_m", "Geography(MultiLineStringM, 4326)"),
    ("geography_multilinestring_z", "Geography(MultiLineStringZ, 4326)"),
    ("geography_multilinestring_zm", "Geography(MultiLineStringZM, 4326)"),
    ("geography_multipolygon", "Geography(MultiPolygon, 4326)"),
    ("geography_multipolygon_m", "Geography(MultiPolygonM, 4326)"),
    ("geography_multipolygon_z", "Geography(MultiPolygonZ, 4326)"),
    ("geography_multipolygon_zm", "Geography(MultiPolygonZM, 4326)"),
    ("geography_geometrycollection", "Geography(GeometryCollection, 4326)"),
    ("geography_geometrycollection_m", "Geography(GeometryCollectionM, 4326)"),
    ("geography_geometrycollection_z", "Geography(GeometryCollectionZ, 4326)"),
    (
        "geography_geometrycollection_zm",
        "Geography(GeometryCollectionZM, 4326)",
    ),
];

#[test_connector(tags(Postgres), exclude(PostGIS, CockroachDb))]
async fn native_type_columns_feature_on(api: &mut TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("\"{name}\" {db_type} Not Null"))
        .collect();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id Integer Primary Key");
                for column in &columns {
                    t.inject_custom(column);
                }
            });
        })
        .await?;

    let types = indoc! {r#"
        model Blog {
            id              Int      @id
            smallint        Int      @db.SmallInt
            int             Int
            bigint          BigInt
            decimal         Decimal  @db.Decimal(4, 2)
            numeric         Decimal  @db.Decimal(4, 2)
            real            Float    @db.Real
            doublePrecision Float
            smallSerial     Int      @default(autoincrement()) @db.SmallInt
            serial          Int      @default(autoincrement())
            bigSerial       BigInt   @default(autoincrement())
            varChar         String   @db.VarChar(200)
            char            String   @db.Char(200)
            text            String
            bytea           Bytes
            ts              DateTime @db.Timestamp(0)
            tstz            DateTime @db.Timestamptz(2)
            date            DateTime @db.Date
            time            DateTime @db.Time(2)
            time_2          DateTime @db.Time(6)
            timetz          DateTime @db.Timetz(2)
            bool            Boolean
            bit             String   @db.Bit(1)
            varbit          String   @db.VarBit(1)
            uuid            String   @db.Uuid
            xml             String   @db.Xml
            json            Json     @db.Json
            jsonb           Json
            money           Decimal  @db.Money
            oid             Int      @db.Oid
            inet            String   @db.Inet
          }
    "#};

    let result = api.introspect().await?;

    println!("EXPECTATION: \n {types:#}");
    println!("RESULT: \n {result:#}");

    api.assert_eq_datamodels(types, &result);

    Ok(())
}

#[test_connector(tags(PostGIS))]
async fn native_type_spatial_columns_feature_on(api: &mut TestApi) -> TestResult {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis").await;

    let columns: Vec<String> = GEOMETRY_TYPES
        .iter()
        .map(|(name, db_type)| format!("\"{name}\" {db_type} Not Null"))
        .collect();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Spatial", move |t| {
                t.inject_custom("id Integer Primary Key");
                for column in &columns {
                    t.inject_custom(column);
                }
            });
        })
        .await?;

    let mut types = indoc! {r#"
         model Spatial {
            id                              Int      @id
            geometry                        Geometry
            geometry_srid                   Geometry @db.Geometry(Geometry, 3857)
            geometry_geometry_m             Geometry @db.Geometry(GeometryM)
            geometry_geometry_z             Geometry @db.Geometry(GeometryZ)
            geometry_geometry_zm            Geometry @db.Geometry(GeometryZM)
            geometry_point                  Geometry @db.Geometry(Point)
            geometry_point_m                Geometry @db.Geometry(PointM)
            geometry_point_z                Geometry @db.Geometry(PointZ)
            geometry_point_zm               Geometry @db.Geometry(PointZM)
            geometry_linestring             Geometry @db.Geometry(LineString)
            geometry_linestring_m           Geometry @db.Geometry(LineStringM)
            geometry_linestring_z           Geometry @db.Geometry(LineStringZ)
            geometry_linestring_zm          Geometry @db.Geometry(LineStringZM)
            geometry_polygon                Geometry @db.Geometry(Polygon)
            geometry_polygon_m              Geometry @db.Geometry(PolygonM)
            geometry_polygon_z              Geometry @db.Geometry(PolygonZ)
            geometry_polygon_zm             Geometry @db.Geometry(PolygonZM)
            geometry_multipoint             Geometry @db.Geometry(MultiPoint)
            geometry_multipoint_m           Geometry @db.Geometry(MultiPointM)
            geometry_multipoint_z           Geometry @db.Geometry(MultiPointZ)
            geometry_multipoint_zm          Geometry @db.Geometry(MultiPointZM)
            geometry_multilinestring        Geometry @db.Geometry(MultiLineString)
            geometry_multilinestring_m      Geometry @db.Geometry(MultiLineStringM)
            geometry_multilinestring_z      Geometry @db.Geometry(MultiLineStringZ)
            geometry_multilinestring_zm     Geometry @db.Geometry(MultiLineStringZM)
            geometry_multipolygon           Geometry @db.Geometry(MultiPolygon)
            geometry_multipolygon_m         Geometry @db.Geometry(MultiPolygonM)
            geometry_multipolygon_z         Geometry @db.Geometry(MultiPolygonZ)
            geometry_multipolygon_zm        Geometry @db.Geometry(MultiPolygonZM)
            geometry_geometrycollection     Geometry @db.Geometry(GeometryCollection)
            geometry_geometrycollection_m   Geometry @db.Geometry(GeometryCollectionM)
            geometry_geometrycollection_z   Geometry @db.Geometry(GeometryCollectionZ)
            geometry_geometrycollection_zm  Geometry @db.Geometry(GeometryCollectionZM)
            geography_geometry              Geometry @db.Geography(Geometry, 4326)
            geography_geometry_m            Geometry @db.Geography(GeometryM, 4326)
            geography_geometry_z            Geometry @db.Geography(GeometryZ, 4326)
            geography_geometry_zm           Geometry @db.Geography(GeometryZM, 4326)
            geography_point                 Geometry @db.Geography(Point, 4326)
            geography_point_m               Geometry @db.Geography(PointM, 4326)
            geography_point_z               Geometry @db.Geography(PointZ, 4326)
            geography_point_zm              Geometry @db.Geography(PointZM, 4326)
            geography_linestring            Geometry @db.Geography(LineString, 4326)
            geography_linestring_m          Geometry @db.Geography(LineStringM, 4326)
            geography_linestring_z          Geometry @db.Geography(LineStringZ, 4326)
            geography_linestring_zm         Geometry @db.Geography(LineStringZM, 4326)
            geography_polygon               Geometry @db.Geography(Polygon, 4326)
            geography_polygon_m             Geometry @db.Geography(PolygonM, 4326)
            geography_polygon_z             Geometry @db.Geography(PolygonZ, 4326)
            geography_polygon_zm            Geometry @db.Geography(PolygonZM, 4326)
            geography_multipoint            Geometry @db.Geography(MultiPoint, 4326)
            geography_multipoint_m          Geometry @db.Geography(MultiPointM, 4326)
            geography_multipoint_z          Geometry @db.Geography(MultiPointZ, 4326)
            geography_multipoint_zm         Geometry @db.Geography(MultiPointZM, 4326)
            geography_multilinestring       Geometry @db.Geography(MultiLineString, 4326)
            geography_multilinestring_m     Geometry @db.Geography(MultiLineStringM, 4326)
            geography_multilinestring_z     Geometry @db.Geography(MultiLineStringZ, 4326)
            geography_multilinestring_zm    Geometry @db.Geography(MultiLineStringZM, 4326)
            geography_multipolygon          Geometry @db.Geography(MultiPolygon, 4326)
            geography_multipolygon_m        Geometry @db.Geography(MultiPolygonM, 4326)
            geography_multipolygon_z        Geometry @db.Geography(MultiPolygonZ, 4326)
            geography_multipolygon_zm       Geometry @db.Geography(MultiPolygonZM, 4326)
            geography_geometrycollection    Geometry @db.Geography(GeometryCollection, 4326)
            geography_geometrycollection_m  Geometry @db.Geography(GeometryCollectionM, 4326)
            geography_geometrycollection_z  Geometry @db.Geography(GeometryCollectionZ, 4326)
            geography_geometrycollection_zm Geometry @db.Geography(GeometryCollectionZM, 4326)
        }
    "#}
    .to_string();

    // TODO@geometry: shouldn't spatial_ref_sys be ignored here ?
    if !api.is_cockroach() {
        types += indoc!(
            r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model spatial_ref_sys {
          srid      Int     @id
          auth_name String? @db.VarChar(256)
          auth_srid Int?
          srtext    String? @db.VarChar(2048)
          proj4text String? @db.VarChar(2048)
        }
    "#
        );
    }

    let result = api.introspect().await?;

    println!("EXPECTATION: \n {types:#}");
    println!("RESULT: \n {result:#}");

    api.assert_eq_datamodels(&types, &result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(PostGIS, CockroachDb))]
async fn native_type_array_columns_feature_on(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id Integer Primary Key");
                t.inject_custom("decimal_array Decimal(42,0)[] ");
                t.inject_custom("decimal_array_2 Decimal[] ");
                t.inject_custom("numeric_array Numeric(4, 2)[] ");
                t.inject_custom("numeric_array_2 Numeric[] ");
                t.inject_custom("varchar_array Varchar(42)[] ");
                t.inject_custom("varchar_array_2 Varchar[] ");
                t.inject_custom("char_array Char(200)[] ");
                t.inject_custom("char_array_2 Char[] ");
                t.inject_custom("bit_array Bit(20)[] ");
                t.inject_custom("bit_array_2 Bit[] ");
                t.inject_custom("varbit_array Varbit(2)[] ");
                t.inject_custom("varbit_array_2 Varbit[] ");
                t.inject_custom("timestamp_array Timestamp(4)[] ");
                t.inject_custom("time_array Time(4)[] ");
            });
        })
        .await?;

    let types = indoc! {r#"
         model Blog {
          id                Int        @id
          decimal_array     Decimal[]  @db.Decimal(42, 0)
          decimal_array_2   Decimal[]  @db.Decimal
          numeric_array     Decimal[]  @db.Decimal(4, 2)
          numeric_array_2   Decimal[]  @db.Decimal
          varchar_array     String[]   @db.VarChar(42)
          varchar_array_2   String[]   @db.VarChar
          char_array        String[]   @db.Char(200)
          char_array_2      String[]   @db.Char(1)
          bit_array         String[]   @db.Bit(20)
          bit_array_2       String[]   @db.Bit(1)
          varbit_array      String[]   @db.VarBit(2)
          varbit_array_2    String[]   @db.VarBit
          timestamp_array   DateTime[] @db.Timestamp(4)
          time_array        DateTime[] @db.Time(4)
        }
    "#}
    .to_string();

    let result = api.introspect().await?;

    println!("EXPECTATION: \n {types:#}");
    println!("RESULT: \n {result:#}");

    api.assert_eq_datamodels(&types, &result);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn cdb_char_is_a_char(api: &mut TestApi) -> TestResult {
    // https://github.com/prisma/prisma/issues/12281

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id Integer Primary Key");
                t.inject_custom(r#"ch "char" DEFAULT 'Y'::"char" NOT NULL"#);
            });
        })
        .await?;

    let result = api.introspect_dml().await?;

    let expected = expect![[r#"
        model Blog {
          id Int    @id
          ch String @default("Y") @db.CatalogSingleChar
        }
    "#]];

    expected.assert_eq(&result);

    Ok(())
}
