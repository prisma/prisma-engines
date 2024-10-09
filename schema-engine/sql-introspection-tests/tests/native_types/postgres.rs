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
    ("geometry_default", "Geometry(Geometry, 4326)"),
    ("geometry_srid", "Geometry(Geometry, 3857)"),
    ("geometry_geometry_z", "Geometry(GeometryZ)"),
    ("geometry_point", "Geometry(Point)"),
    ("geometry_point_z", "Geometry(PointZ)"),
    ("geometry_linestring", "Geometry(LineString)"),
    ("geometry_linestring_z", "Geometry(LineStringZ)"),
    ("geometry_polygon", "Geometry(Polygon)"),
    ("geometry_polygon_z", "Geometry(PolygonZ)"),
    ("geometry_multipoint", "Geometry(MultiPoint)"),
    ("geometry_multipoint_z", "Geometry(MultiPointZ)"),
    ("geometry_multilinestring", "Geometry(MultiLineString)"),
    ("geometry_multilinestring_z", "Geometry(MultiLineStringZ)"),
    ("geometry_multipolygon", "Geometry(MultiPolygon)"),
    ("geometry_multipolygon_z", "Geometry(MultiPolygonZ)"),
    ("geometry_geometrycollection", "Geometry(GeometryCollection)"),
    ("geometry_geometrycollection_z", "Geometry(GeometryCollectionZ)"),
    ("geography_geometry", "Geography(Geometry, 9000)"),
    ("geography_geometry_z", "Geography(GeometryZ, 9000)"),
    ("geography_point", "Geography(Point, 9000)"),
    ("geography_point_z", "Geography(PointZ, 9000)"),
    ("geography_linestring", "Geography(LineString, 9000)"),
    ("geography_linestring_z", "Geography(LineStringZ, 9000)"),
    ("geography_polygon", "Geography(Polygon, 9000)"),
    ("geography_polygon_z", "Geography(PolygonZ, 9000)"),
    ("geography_multipoint", "Geography(MultiPoint, 9000)"),
    ("geography_multipoint_z", "Geography(MultiPointZ, 9000)"),
    ("geography_multilinestring", "Geography(MultiLineString, 9000)"),
    ("geography_multilinestring_z", "Geography(MultiLineStringZ, 9000)"),
    ("geography_multipolygon", "Geography(MultiPolygon, 9000)"),
    ("geography_multipolygon_z", "Geography(MultiPolygonZ, 9000)"),
    ("geography_geometrycollection", "Geography(GeometryCollection, 9000)"),
    ("geography_geometrycollection_z", "Geography(GeometryCollectionZ, 9000)"),
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
            geometry_default                Geometry
            geometry_srid                   Geometry @db.Geometry(Geometry, 3857)
            geometry_geometry_z             Geometry @db.Geometry(GeometryZ, 0)
            geometry_point                  Geometry @db.Geometry(Point, 0)
            geometry_point_z                Geometry @db.Geometry(PointZ, 0)
            geometry_linestring             Geometry @db.Geometry(LineString, 0)
            geometry_linestring_z           Geometry @db.Geometry(LineStringZ, 0)
            geometry_polygon                Geometry @db.Geometry(Polygon, 0)
            geometry_polygon_z              Geometry @db.Geometry(PolygonZ, 0)
            geometry_multipoint             Geometry @db.Geometry(MultiPoint, 0)
            geometry_multipoint_z           Geometry @db.Geometry(MultiPointZ, 0)
            geometry_multilinestring        Geometry @db.Geometry(MultiLineString, 0)
            geometry_multilinestring_z      Geometry @db.Geometry(MultiLineStringZ, 0)
            geometry_multipolygon           Geometry @db.Geometry(MultiPolygon, 0)
            geometry_multipolygon_z         Geometry @db.Geometry(MultiPolygonZ, 0)
            geometry_geometrycollection     Geometry @db.Geometry(GeometryCollection, 0)
            geometry_geometrycollection_z   Geometry @db.Geometry(GeometryCollectionZ, 0)
            geography_geometry              Geometry @db.Geography(Geometry, 9000)
            geography_geometry_z            Geometry @db.Geography(GeometryZ, 9000)
            geography_point                 Geometry @db.Geography(Point, 9000)
            geography_point_z               Geometry @db.Geography(PointZ, 9000)
            geography_linestring            Geometry @db.Geography(LineString, 9000)
            geography_linestring_z          Geometry @db.Geography(LineStringZ, 9000)
            geography_polygon               Geometry @db.Geography(Polygon, 9000)
            geography_polygon_z             Geometry @db.Geography(PolygonZ, 9000)
            geography_multipoint            Geometry @db.Geography(MultiPoint, 9000)
            geography_multipoint_z          Geometry @db.Geography(MultiPointZ, 9000)
            geography_multilinestring       Geometry @db.Geography(MultiLineString, 9000)
            geography_multilinestring_z     Geometry @db.Geography(MultiLineStringZ, 9000)
            geography_multipolygon          Geometry @db.Geography(MultiPolygon, 9000)
            geography_multipolygon_z        Geometry @db.Geography(MultiPolygonZ, 9000)
            geography_geometrycollection    Geometry @db.Geography(GeometryCollection, 9000)
            geography_geometrycollection_z  Geometry @db.Geography(GeometryCollectionZ, 9000)
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
