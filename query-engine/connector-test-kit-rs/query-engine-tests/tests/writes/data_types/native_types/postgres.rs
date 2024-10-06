use query_engine_tests::*;

#[test_suite(only(Postgres))]
mod postgres {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_int() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              int      Int    @test.Integer
              sInt     Int    @test.SmallInt
              bInt     BigInt @test.BigInt
              oid      Int    @test.Oid
              inc_int  Int    @test.Integer     @default(autoincrement())
              inc_sInt Int    @test.SmallInt    @default(autoincrement())
              inc_bInt BigInt @test.BigInt      @default(autoincrement())
            }"#
        };

        schema.to_owned()
    }

    //"Postgres native int types" should "work"
    #[connector_test(schema(schema_int), only(Postgres))]
    async fn native_int_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                int: 2147483647
                sInt: 32767
                bInt: "9223372036854775807"
                oid: 0
              }
            ) {
              int
              sInt
              bInt
              oid
              inc_int
              inc_sInt
              inc_bInt
            }
          }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"bInt":"9223372036854775807","oid":0,"inc_int":1,"inc_sInt":1,"inc_bInt":"1"}}}"###
        );

        Ok(())
    }

    fn schema_decimal() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              float    Float   @test.Real
              dfloat   Float   @test.DoublePrecision
              decFloat Decimal @test.Decimal(2, 1)
              money    Decimal @test.Money
            }"#
        };

        schema.to_owned()
    }

    // "Postgres native decimal types" should "work"
    #[connector_test(schema(schema_decimal), only(Postgres), exclude(CockroachDb))]
    async fn native_decimal_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                float: 1.1
                dfloat: 2.2
                decFloat: 3.1234
                money: 3.51
              }
            ) {
              float
              dfloat
              decFloat
              money
            }
          }"#),
          // decFloat is cut due to precision
          @r###"{"data":{"createOneModel":{"float":1.1,"dfloat":2.2,"decFloat":"3.1","money":"3.51"}}}"###
        );

        Ok(())
    }

    fn schema_decimal_cockroach() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              float    Float   @test.Float4
              dfloat   Float   @test.Float8
              decFloat Decimal @test.Decimal(2, 1)
            }"#
        };

        schema.to_owned()
    }

    // Cockroach does not support money.
    #[connector_test(schema(schema_decimal_cockroach), only(CockroachDb))]
    async fn native_decimal_types_cockroach(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                float: 1.1
                dfloat: 2.2
                decFloat: 3.1234
              }
            ) {
              float
              dfloat
              decFloat
            }
          }"#),
          // decFloat is cut due to precision
          @r###"{"data":{"createOneModel":{"float":1.1,"dfloat":2.2,"decFloat":"3.1"}}}"###
        );

        Ok(())
    }

    fn schema_string() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              char  String @test.Char(10)
              vChar String @test.VarChar(11)
              text  String @test.Text
              bit   String @test.Bit(4)
              vBit  String @test.VarBit(5)
              uuid  String @test.Uuid
              ip    String @test.Inet
            }"#
        };

        schema.to_owned()
    }

    // "Postgres native string types" should "work"
    #[connector_test(schema(schema_string))]
    async fn native_string(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                char: "1234567890"
                vChar: "12345678910"
                text: "text"
                bit: "1010"
                vBit: "00110"
                uuid: "123e4567-e89b-12d3-a456-426614174000"
                ip: "127.0.0.1"
              }
            ) {
              char
              vChar
              text
              bit
              vBit
              uuid
              ip
            }
          }"#),
          @r###"{"data":{"createOneModel":{"char":"1234567890","vChar":"12345678910","text":"text","bit":"1010","vBit":"00110","uuid":"123e4567-e89b-12d3-a456-426614174000","ip":"127.0.0.1"}}}"###
        );

        Ok(())
    }

    fn schema_other_types() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              bool  Boolean @test.Boolean
              byteA Bytes   @test.ByteA
              xml   String  @test.Xml
              json  Json    @test.Json
              jsonb Json    @test.JsonB
            }"#
        };

        schema.to_owned()
    }

    // "Other Postgres native types" should "work"
    #[connector_test(schema(schema_other_types), only(Postgres), exclude(CockroachDb,))]
    async fn native_other_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                bool: true
                byteA: "dGVzdA=="
                xml: "<wurst>salat</wurst>"
                json: "{}"
                jsonb: "{\"a\": \"b\"}"
              }
            ) {
              bool
              byteA
              xml
              json
              jsonb
            }
          }"#),
          @r###"{"data":{"createOneModel":{"bool":true,"byteA":"dGVzdA==","xml":"<wurst>salat</wurst>","json":"{}","jsonb":"{\"a\":\"b\"}"}}}"###
        );

        Ok(())
    }

    fn schema_other_types_cockroach() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              bool  Boolean @test.Bool
              byteA Bytes   @test.Bytes
              jsonb Json    @test.JsonB
            }"#
        };

        schema.to_owned()
    }

    // Cockroach does not support XML.
    #[connector_test(schema(schema_other_types_cockroach), only(CockroachDb))]
    async fn native_other_types_cockroach(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                bool: true
                byteA: "dGVzdA=="
                jsonb: "{\"a\": \"b\"}"
              }
            ) {
              bool
              byteA
              jsonb
            }
          }"#),
          @r###"{"data":{"createOneModel":{"bool":true,"byteA":"dGVzdA==","jsonb":"{\"a\":\"b\"}"}}}"###
        );

        Ok(())
    }

    fn schema_date() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              date       DateTime @test.Date
              date_2     DateTime @test.Date
              time       DateTime @test.Time(3)
              time_2     DateTime @test.Time(3)
              time_tz    DateTime @test.Timetz(3)
              time_tz_2  DateTime @test.Timetz(3)
              ts         DateTime @test.Timestamp(3)
              ts_2       DateTime @test.Timestamp(3)
              ts_tz      DateTime @test.Timestamptz(3)
              ts_tz_2    DateTime @test.Timestamptz(3)
            }"#
        };

        schema.to_owned()
    }

    // "Postgres native date types" should "work"
    #[connector_test(schema(schema_date))]
    async fn native_date(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                date: "2016-09-24T00:00:00.000Z"
                date_2: "2016-09-24T00:00:00.000+03:00"
                time: "1111-11-11T13:02:20.321Z"
                time_2: "1111-11-11T13:02:20.321+03:00"
                time_tz: "1111-11-11T13:02:20.321Z"
                time_tz_2: "1111-11-11T13:02:20.321+03:00"
                ts: "2016-09-24T14:01:30.213Z"
                ts_2: "2016-09-24T14:01:30.213+03:00"
                ts_tz: "2016-09-24T14:01:30.213Z"
                ts_tz_2: "2016-09-24T14:01:30.213+03:00"
              }
            ) {
              date
              date_2
              time
              time_2
              time_tz
              time_tz_2
              ts
              ts_2
              ts_tz
              ts_tz_2
            }
          }"#),
          @r###"{"data":{"createOneModel":{"date":"2016-09-24T00:00:00.000Z","date_2":"2016-09-23T00:00:00.000Z","time":"1970-01-01T13:02:20.321Z","time_2":"1970-01-01T10:02:20.321Z","time_tz":"1970-01-01T13:02:20.321Z","time_tz_2":"1970-01-01T10:02:20.321Z","ts":"2016-09-24T14:01:30.213Z","ts_2":"2016-09-24T11:01:30.213Z","ts_tz":"2016-09-24T14:01:30.213Z","ts_tz_2":"2016-09-24T11:01:30.213Z"}}}"###
        );

        Ok(())
    }

    fn schema_native_fixed_size_char() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id, @test.Char(16))
              b_id String? @unique @test.Char(16)
              b    ModelB? @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, String, @id, @test.Char(16))
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Postgres native fixed-size char type" should "be handled correctly wrt. padding for comparisons"
    #[connector_test(schema(schema_native_fixed_size_char))]
    async fn native_fixed_size_char(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: "1234"
               b: { create: { id: "4321" } }
            }) {
              id
              b { id }
            }
          }"#),
          // This is correct - postgres returns padded strings (as opposed to MySQL for example, where it's trimmed).
          @r###"{"data":{"createOneModelA":{"id":"1234            ","b":{"id":"4321            "}}}}"###
        );

        Ok(())
    }

    fn schema_geometry() -> String {
        let schema = indoc! {
            r#"model Model {
            @@schema("test")
            #id(id, String, @id, @default(cuid()))
            geometry             Geometry @test.Geometry(Geometry)
            geometry_point       Geometry @test.Geometry(Point)
            geometry_line        Geometry @test.Geometry(LineString)
            geometry_poly        Geometry @test.Geometry(Polygon)
            geometry_multipoint  Geometry @test.Geometry(MultiPoint)
            geometry_multiline   Geometry @test.Geometry(MultiLineString)
            geometry_multipoly   Geometry @test.Geometry(MultiPolygon)
            geometry_collection  Geometry @test.Geometry(GeometryCollection)
          }"#
        };

        schema.to_owned()
    }

    // "PostGIS common geometry types" should "work"
    #[connector_test(
        only(Postgres("16-postgis"), CockroachDb),
        schema(schema_geometry),
        db_schemas("public", "test")
    )]
    async fn native_geometry(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              geometry: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              geometry_point: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              geometry_line: "{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]]}"
              geometry_poly: "{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}"
              geometry_multipoint: "{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]]}"
              geometry_multiline: "{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]]}"
              geometry_multipoly: "{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}"
              geometry_collection: "{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"
            }
          ) {
            geometry
            geometry_point
            geometry_line
            geometry_poly
            geometry_multipoint
            geometry_multiline
            geometry_multipoly
            geometry_collection
          }
        }"#),
            @r###"{"data":{"createOneModel":{"geometry":"{\"type\":\"Point\",\"coordinates\":[1,2]}","geometry_point":"{\"type\":\"Point\",\"coordinates\":[1,2]}","geometry_line":"{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]]}","geometry_poly":"{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}","geometry_multipoint":"{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]]}","geometry_multiline":"{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]]}","geometry_multipoly":"{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}","geometry_collection":"{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"}}}"###
        );

        Ok(())
    }

    fn schema_geometry_srid() -> String {
        let schema = indoc! {
            r#"model Model {
            @@schema("test")
            #id(id, String, @id, @default(cuid()))
            geometry             Geometry @test.Geometry(Geometry, 3857)
            geometry_point       Geometry @test.Geometry(Point, 3857)
            geometry_line        Geometry @test.Geometry(LineString, 3857)
            geometry_poly        Geometry @test.Geometry(Polygon, 3857)
            geometry_multipoint  Geometry @test.Geometry(MultiPoint, 3857)
            geometry_multiline   Geometry @test.Geometry(MultiLineString, 3857)
            geometry_multipoly   Geometry @test.Geometry(MultiPolygon, 3857)
            geometry_collection  Geometry @test.Geometry(GeometryCollection, 3857)
          }"#
        };

        schema.to_owned()
    }

    // "PostGIS common geometry types with srid" should "work"
    #[connector_test(
        only(Postgres("16-postgis"), CockroachDb),
        schema(schema_geometry_srid),
        db_schemas("public", "test")
    )]

    async fn native_geometry_srid(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              geometry: "{\"type\":\"Point\",\"coordinates\":[1,2],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
              geometry_point: "{\"type\":\"Point\",\"coordinates\":[1,2],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
              geometry_line: "{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
              geometry_poly: "{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
              geometry_multipoint: "{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
              geometry_multiline: "{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
              geometry_multipoly: "{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
              geometry_collection: "{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}}}"
            }
          ) {
            geometry
            geometry_point
            geometry_line
            geometry_poly
            geometry_multipoint
            geometry_multiline
            geometry_multipoly
            geometry_collection
          }
        }"#),
            @r###"{"data":{"createOneModel":{"geometry":"{\"type\":\"Point\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"coordinates\":[1,2]}","geometry_point":"{\"type\":\"Point\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"coordinates\":[1,2]}","geometry_line":"{\"type\":\"LineString\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"coordinates\":[[1,2],[3,4]]}","geometry_poly":"{\"type\":\"Polygon\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}","geometry_multipoint":"{\"type\":\"MultiPoint\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"coordinates\":[[1,2]]}","geometry_multiline":"{\"type\":\"MultiLineString\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"coordinates\":[[[1,2],[3,4]]]}","geometry_multipoly":"{\"type\":\"MultiPolygon\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}","geometry_collection":"{\"type\":\"GeometryCollection\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:3857\"}},\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"}}}"###
        );

        Ok(())
    }

    fn schema_geography() -> String {
        let schema = indoc! {
            r#"model Model {
            @@schema("test")
            #id(id, String, @id, @default(cuid()))
            geography            Geometry @test.Geography(Geometry)
            geography_point      Geometry @test.Geography(Point)
            geography_line       Geometry @test.Geography(LineString)
            geography_poly       Geometry @test.Geography(Polygon)
            geography_multipoint Geometry @test.Geography(MultiPoint)
            geography_multiline  Geometry @test.Geography(MultiLineString)
            geography_multipoly  Geometry @test.Geography(MultiPolygon)
            geography_collection Geometry @test.Geography(GeometryCollection)
          }"#
        };

        schema.to_owned()
    }

    // "PostGIS common geography types" should "work"
    #[connector_test(
        only(Postgres("16-postgis"), CockroachDb),
        schema(schema_geography),
        db_schemas("public", "test")
    )]
    async fn native_geography(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              geography: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              geography_point: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              geography_line: "{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]]}"
              geography_poly: "{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}"
              geography_multipoint: "{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]]}"
              geography_multiline: "{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]]}"
              geography_multipoly: "{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}"
              geography_collection: "{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"
            }
          ) {
            geography
            geography_point
            geography_line
            geography_poly
            geography_multipoint
            geography_multiline
            geography_multipoly
            geography_collection
          }
        }"#),
            @r###"{"data":{"createOneModel":{"geography":"{\"type\":\"Point\",\"coordinates\":[1,2]}","geography_point":"{\"type\":\"Point\",\"coordinates\":[1,2]}","geography_line":"{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]]}","geography_poly":"{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}","geography_multipoint":"{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]]}","geography_multiline":"{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]]}","geography_multipoly":"{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}","geography_collection":"{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"}}}"###
        );

        Ok(())
    }

    fn schema_geography_srid() -> String {
        let schema = indoc! {
            r#"model Model {
            @@schema("test")
            #id(id, String, @id, @default(cuid()))
            geography            Geometry @test.Geography(Geometry, 9000)
            geography_point      Geometry @test.Geography(Point, 9000)
            geography_line       Geometry @test.Geography(LineString, 9000)
            geography_poly       Geometry @test.Geography(Polygon, 9000)
            geography_multipoint Geometry @test.Geography(MultiPoint, 9000)
            geography_multiline  Geometry @test.Geography(MultiLineString, 9000)
            geography_multipoly  Geometry @test.Geography(MultiPolygon, 9000)
            geography_collection Geometry @test.Geography(GeometryCollection, 9000)
          }"#
        };

        schema.to_owned()
    }

    // "PostGIS common geography types with srid" should "work"
    #[connector_test(
        only(Postgres("16-postgis"), CockroachDb),
        schema(schema_geography_srid),
        db_schemas("public", "test")
    )]
    async fn native_geography_srid(runner: Runner) -> TestResult<()> {
        match_connector_result!(
          &runner,
          r#"mutation {
          createOneModel(
            data: {
              geography: "{\"type\":\"Point\",\"coordinates\":[1,2],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
              geography_point: "{\"type\":\"Point\",\"coordinates\":[1,2],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
              geography_line: "{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
              geography_poly: "{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
              geography_multipoint: "{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
              geography_multiline: "{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
              geography_multipoly: "{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
              geography_collection: "{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}],\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}}}"
            }
          ) {
            geography
            geography_point
            geography_line
            geography_poly
            geography_multipoint
            geography_multiline
            geography_multipoly
            geography_collection
          }
        }"#,
          CockroachDb(_) => r###"{"data":{"createOneModel":{"geography":"{\"type\":\"Point\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"coordinates\":[1,2]}","geography_point":"{\"type\":\"Point\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"coordinates\":[1,2]}","geography_line":"{\"type\":\"LineString\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"coordinates\":[[1,2],[3,4]]}","geography_poly":"{\"type\":\"Polygon\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}","geography_multipoint":"{\"type\":\"MultiPoint\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"coordinates\":[[1,2]]}","geography_multiline":"{\"type\":\"MultiLineString\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"coordinates\":[[[1,2],[3,4]]]}","geography_multipoly":"{\"type\":\"MultiPolygon\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}","geography_collection":"{\"type\":\"GeometryCollection\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"EPSG:9000\"}},\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"}}}"###,
          _ => r###"{"data":{"createOneModel":{"geography":"{\"type\":\"Point\",\"coordinates\":[1,2]}","geography_point":"{\"type\":\"Point\",\"coordinates\":[1,2]}","geography_line":"{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]]}","geography_poly":"{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}","geography_multipoint":"{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]]}","geography_multiline":"{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]]}","geography_multipoly":"{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}","geography_collection":"{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"}}}"###
        );

        Ok(())
    }
}
