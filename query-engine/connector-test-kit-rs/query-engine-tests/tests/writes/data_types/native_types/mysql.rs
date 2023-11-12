use query_engine_tests::*;

#[test_suite(only(Mysql))]
mod mysql {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_int_int() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(inc_int, Int, @id, @default(autoincrement()) @test.Int)
              int  Int    @test.Int
              sInt Int    @test.SmallInt
              mInt Int    @test.MediumInt
              bInt BigInt @test.BigInt
            }"#
        };

        schema.to_owned()
    }

    fn schema_int_smallint() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(inc_sInt, Int, @id, @default(autoincrement()) @test.SmallInt)
            int  Int    @test.Int
            sInt Int    @test.SmallInt
            mInt Int    @test.MediumInt
            bInt BigInt @test.BigInt
          }"#
        };

        schema.to_owned()
    }

    fn schema_int_mediumint() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(inc_mInt, Int, @id, @default(autoincrement()) @test.MediumInt)
            int  Int    @test.Int
            sInt Int    @test.SmallInt
            mInt Int    @test.MediumInt
            bInt BigInt @test.BigInt
          }"#
        };

        schema.to_owned()
    }

    fn schema_bigint_bigint() -> String {
        let schema = indoc! {
            r#"model Model {
          #id(inc_bInt, BigInt, @id, @default(autoincrement()) @test.BigInt)
          int  Int    @test.Int
          sInt Int    @test.SmallInt
          mInt Int    @test.MediumInt
          bInt BigInt @test.BigInt
        }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_int_int))]
    async fn native_int_int(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                int: 2147483647
                sInt: 32767
                mInt: 8388607
                bInt: 5294967295
              }
            ) {
              int
              sInt
              mInt
              bInt
              inc_int
            }
          }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"mInt":8388607,"bInt":"5294967295","inc_int":1}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_int_smallint))]
    async fn native_int_smallint(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                int: 2147483647
                sInt: 32767
                mInt: 8388607
                bInt: 5294967295
              }
            ) {
              int
              sInt
              mInt
              bInt
              inc_sInt
            }
          }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"mInt":8388607,"bInt":"5294967295","inc_sInt":1}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_int_mediumint))]
    async fn native_int_mediumint(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                int: 2147483647
                sInt: 32767
                mInt: 8388607
                bInt: 5294967295
              }
            ) {
              int
              sInt
              mInt
              bInt
              inc_mInt
            }
          }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"mInt":8388607,"bInt":"5294967295","inc_mInt":1}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_bigint_bigint))]
    async fn native_bigint_bigint(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                int: 2147483647
                sInt: 32767
                mInt: 8388607
                bInt: 5294967295
              }
            ) {
              int
              sInt
              mInt
              bInt
              inc_bInt
            }
          }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"mInt":8388607,"bInt":"5294967295","inc_bInt":"1"}}}"###
        );

        Ok(())
    }

    fn schema_decimal() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              float    Float   @test.Float
              dfloat   Float   @test.Double
              decFloat Decimal @test.Decimal(2, 1)
            }"#
        };

        schema.to_owned()
    }

    //"MySQL native decimal types" should "work"
    #[connector_test(schema(schema_decimal))]
    async fn native_decimal_type(runner: Runner) -> TestResult<()> {
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

    fn schema_decimal_vitess() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(id, String, @id, @default(cuid()))
            decLarge Decimal @test.Decimal(20, 10)
          }"#
        };

        schema.to_owned()
    }

    #[connector_test(only(Vitess), schema(schema_decimal_vitess))]
    async fn native_decimal_vitess_precision(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                decLarge: "131603421.38724228"
              }
            ) {
              decLarge
            }
          }"#),
          @r###"{"data":{"createOneModel":{"decLarge":"131603421.38724228"}}}"###
        );

        Ok(())
    }

    fn schema_string() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              char  String @test.Char(10)
              vChar String @test.VarChar(11)
              tText String @test.TinyText
              text  String @test.Text
              mText String @test.MediumText
              ltext String @test.LongText
            }"#
        };

        schema.to_owned()
    }

    // "MySQL native string types" should "work"
    #[connector_test(schema(schema_string))]
    async fn native_string_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                char: "1234567890"
                vChar: "12345678910"
                tText: "tiny text"
                text: "text"
                mText: "medium text"
                ltext: "long text"
              }
            ) {
              char
              vChar
              tText
              text
              mText
              ltext
            }
          }"#),
          @r###"{"data":{"createOneModel":{"char":"1234567890","vChar":"12345678910","tText":"tiny text","text":"text","mText":"medium text","ltext":"long text"}}}"###
        );

        Ok(())
    }

    fn schema_date_types() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              date  DateTime @test.Date
              time  DateTime @test.Time(5)
              dtime DateTime @test.DateTime
              ts    DateTime @test.Timestamp
              year  Int      @test.Year
            }"#
        };

        schema.to_owned()
    }

    // "MySQL native date types" should "work"
    #[connector_test(schema(schema_date_types))]
    async fn native_date_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                date: "2016-09-24T00:00:00.000Z"
                time: "2016-09-24T13:14:15.123Z"
                dtime: "2016-09-24T12:29:32.342Z"
                ts: "2016-09-24T12:29:32.342Z"
                year: 1973
              }
            ) {
              date
              time
              dtime
              ts
              year
            }
          }"#),
          @r###"{"data":{"createOneModel":{"date":"2016-09-24T00:00:00.000Z","time":"1970-01-01T13:14:15.123Z","dtime":"2016-09-24T12:29:32.000Z","ts":"2016-09-24T12:29:32.000Z","year":1973}}}"###
        );

        Ok(())
    }

    fn schema_binary() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              bit   Bytes @test.Bit(8)
              bin   Bytes @test.Binary(4)
              vBin  Bytes @test.VarBinary(5)
              blob  Bytes @test.Blob
              tBlob Bytes @test.TinyBlob
              mBlob Bytes @test.MediumBlob
              lBlob Bytes @test.LongBlob
            }"#
        };

        schema.to_owned()
    }

    // "MySQL native binary types" should "work"
    #[connector_test(schema(schema_binary))]
    async fn native_binary_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                bit: "dA=="
                bin: "dGVzdA=="
                vBin: "dGVzdA=="
                blob: "dGVzdA=="
                tBlob: "dGVzdA=="
                mBlob: "dGVzdA=="
                lBlob: "dGVzdA=="
              }
            ) {
              bit
              bin
              vBin
              blob
              tBlob
              mBlob
              lBlob
            }
          }"#),
          @r###"{"data":{"createOneModel":{"bit":"dA==","bin":"dGVzdA==","vBin":"dGVzdA==","blob":"dGVzdA==","tBlob":"dGVzdA==","mBlob":"dGVzdA==","lBlob":"dGVzdA=="}}}"###
        );

        Ok(())
    }

    fn schema_other_native_types() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              tInt Boolean @test.TinyInt
              bit  Boolean @test.Bit(1)
            }"#
        };

        schema.to_owned()
    }

    // "Other MySQL native types" should "work"
    #[connector_test(schema(schema_other_native_types))]
    async fn other_native_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                tInt: true
                bit: true
              }
            ) {
              tInt
              bit
            }
          }"#),
          @r###"{"data":{"createOneModel":{"tInt":true,"bit":true}}}"###
        );

        Ok(())
    }

    fn schema_fixed_size_char_native_types() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id, @test.Char(16))
              b_id String? @test.Char(16) @unique
              b    ModelB? @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, String, @id, @test.Char(16))
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "MySQL fixed-size char native type" should "be handled correctly wrt. padding"
    #[connector_test(schema(schema_fixed_size_char_native_types))]
    async fn fixed_size_char_native_type(runner: Runner) -> TestResult<()> {
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
          @r###"{"data":{"createOneModelA":{"id":"1234","b":{"id":"4321"}}}}"###
        );

        Ok(())
    }

    fn schema_ewkt_geometry_types() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(id, String, @id, @default(cuid()))
            geometry    Geometry @test.Geometry
            point       Geometry @test.Point
            line        Geometry @test.LineString
            poly        Geometry @test.Polygon
            multipoint  Geometry @test.MultiPoint
            multiline   Geometry @test.MultiLineString
            multipoly   Geometry @test.MultiPolygon
            collection  Geometry @test.GeometryCollection
          }"#
        };

        schema.to_owned()
    }

    // "MySQL native spatial types" should "work"
    #[connector_test(schema(schema_ewkt_geometry_types))]
    async fn native_ewkt_geometry_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                geometry: "POINT(1 2)"
                point: "POINT(1 2)"
                line: "LINESTRING(1 2,3 4)"
                poly: "POLYGON((1 2,3 4,5 6,1 2))"
                multipoint: "MULTIPOINT(1 2)"
                multiline: "MULTILINESTRING((1 2,3 4))"
                multipoly: "MULTIPOLYGON(((1 2,3 4,5 6,1 2)))"
                collection: "GEOMETRYCOLLECTION(POINT(1 2))"
              }
            ) {
              geometry,
              point
              line
              poly
              multipoint
              multiline
              multipoly
              collection
            }
          }"#),
          @r###"{"data":{"createOneModel":{"geometry":"POINT(1 2)","point":"POINT(1 2)","line":"LINESTRING(1 2,3 4)","poly":"POLYGON((1 2,3 4,5 6,1 2))","multipoint":"MULTIPOINT(1 2)","multiline":"MULTILINESTRING((1 2,3 4))","multipoly":"MULTIPOLYGON(((1 2,3 4,5 6,1 2)))","collection":"GEOMETRYCOLLECTION(POINT(1 2))"}}}"###
        );

        Ok(())
    }

    fn schema_geojson_geometry_types() -> String {
        let schema = indoc! {
            r#"model Model {
          #id(id, String, @id, @default(cuid()))
          geometry    GeoJson @test.Geometry
          point       GeoJson @test.Point
          line        GeoJson @test.LineString
          poly        GeoJson @test.Polygon
          multipoint  GeoJson @test.MultiPoint
          multiline   GeoJson @test.MultiLineString
          multipoly   GeoJson @test.MultiPolygon
          collection  GeoJson @test.GeometryCollection
        }"#
        };

        schema.to_owned()
    }

    // "MySQL native spatial types" should "work"
    #[connector_test(schema(schema_geojson_geometry_types))]
    async fn native_geojson_geometry_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              geometry: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              point: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              line: "{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]]}"
              poly: "{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}"
              multipoint: "{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]]}"
              multiline: "{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]]}"
              multipoly: "{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}"
              collection: "{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"
            }
          ) {
            geometry,
            point
            line
            poly
            multipoint
            multiline
            multipoly
            collection
          }
        }"#),
          @r###"{"data":{"createOneModel":{"geometry":"{\"type\": \"Point\", \"coordinates\": [1,2]}","point":"{\"type\": \"Point\", \"coordinates\": [1,2]}","line":"{\"type\": \"LineString\", \"coordinates\": [[1,2],[3,4]]}","poly":"{\"type\": \"Polygon\", \"coordinates\": [[[1,2],[3,4],[5,6],[1,2]]]}","multipoint":"{\"type\": \"MultiPoint\", \"coordinates\": [[1,2]]}","multiline":"{\"type\": \"MultiLineString\", \"coordinates\": [[[1,2],[3,4]]]}","multipoly":"{\"type\": \"MultiPolygon\", \"coordinates\": [[[[1,2],[3,4],[5,6],[1,2]]]]}","collection":"{\"type\": \"GeometryCollection\", \"geometries\": [{\"type\": \"Point\", \"coordinates\": [1,2]}]}"}}}"###
        );

        Ok(())
    }

    fn schema_geometry_srid_types() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(id, String, @id, @default(cuid()))
            geometry    Geometry @test.Geometry(4326)
            point       Geometry @test.Point(4326)
            line        Geometry @test.LineString(4326)
            poly        Geometry @test.Polygon(4326)
            multipoint  Geometry @test.MultiPoint(4326)
            multiline   Geometry @test.MultiLineString(4326)
            multipoly   Geometry @test.MultiPolygon(4326)
            collection  Geometry @test.GeometryCollection(4326)
          }"#
        };

        schema.to_owned()
    }

    // "MySQL native spatial srid types" should "work"
    #[connector_test(only(MySQL(8)), schema(schema_geometry_srid_types))]
    async fn native_geometry_srid_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                geometry: "SRID=4326;POINT(1 2)"
                point: "SRID=4326;POINT(1 2)"
                line: "SRID=4326;LINESTRING(1 2,3 4)"
                poly: "SRID=4326;POLYGON((1 2,3 4,5 6,1 2))"
                multipoint: "SRID=4326;MULTIPOINT(1 2)"
                multiline: "SRID=4326;MULTILINESTRING((1 2,3 4))"
                multipoly: "SRID=4326;MULTIPOLYGON(((1 2,3 4,5 6,1 2)))"
                collection: "SRID=4326;GEOMETRYCOLLECTION(POINT(1 2))"
              }
            ) {
              geometry
              point
              line
              poly
              multipoint
              multiline
              multipoly
              collection
            }
          }"#),
          @r###"{"data":{"createOneModel":{"geometry":"SRID=4326;POINT(1 2)","point":"SRID=4326;POINT(1 2)","line":"SRID=4326;LINESTRING(1 2,3 4)","poly":"SRID=4326;POLYGON((1 2,3 4,5 6,1 2))","multipoint":"SRID=4326;MULTIPOINT(1 2)","multiline":"SRID=4326;MULTILINESTRING((1 2,3 4))","multipoly":"SRID=4326;MULTIPOLYGON(((1 2,3 4,5 6,1 2)))","collection":"SRID=4326;GEOMETRYCOLLECTION(POINT(1 2))"}}}"###
        );

        Ok(())
    }

    fn schema_geojson_srid_geometry_types() -> String {
        let schema = indoc! {
            r#"model Model {
          #id(id, String, @id, @default(cuid()))
          geometry    GeoJson @test.Geometry(4326)
          point       GeoJson @test.Point(4326)
          line        GeoJson @test.LineString(4326)
          poly        GeoJson @test.Polygon(4326)
          multipoint  GeoJson @test.MultiPoint(4326)
          multiline   GeoJson @test.MultiLineString(4326)
          multipoly   GeoJson @test.MultiPolygon(4326)
          collection  GeoJson @test.GeometryCollection(4326)
        }"#
        };

        schema.to_owned()
    }

    // "MySQL native spatial types" should "work"
    #[connector_test(only(MySQL(8)), schema(schema_geojson_srid_geometry_types))]
    async fn native_geojson_srid_geometry_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              geometry: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              point: "{\"type\":\"Point\",\"coordinates\":[1,2]}"
              line: "{\"type\":\"LineString\",\"coordinates\":[[1,2],[3,4]]}"
              poly: "{\"type\":\"Polygon\",\"coordinates\":[[[1,2],[3,4],[5,6],[1,2]]]}"
              multipoint: "{\"type\":\"MultiPoint\",\"coordinates\":[[1,2]]}"
              multiline: "{\"type\":\"MultiLineString\",\"coordinates\":[[[1,2],[3,4]]]}"
              multipoly: "{\"type\":\"MultiPolygon\",\"coordinates\":[[[[1,2],[3,4],[5,6],[1,2]]]]}"
              collection: "{\"type\":\"GeometryCollection\",\"geometries\":[{\"type\":\"Point\",\"coordinates\":[1,2]}]}"
            }
          ) {
            geometry,
            point
            line
            poly
            multipoint
            multiline
            multipoly
            collection
          }
        }"#),
          @r###"{"data":{"createOneModel":{"geometry":"{\"type\": \"Point\", \"coordinates\": [1,2]}","point":"{\"type\": \"Point\", \"coordinates\": [1,2]}","line":"{\"type\": \"LineString\", \"coordinates\": [[1,2],[3,4]]}","poly":"{\"type\": \"Polygon\", \"coordinates\": [[[1,2],[3,4],[5,6],[1,2]]]}","multipoint":"{\"type\": \"MultiPoint\", \"coordinates\": [[1,2]]}","multiline":"{\"type\": \"MultiLineString\", \"coordinates\": [[[1,2],[3,4]]]}","multipoly":"{\"type\": \"MultiPolygon\", \"coordinates\": [[[[1,2],[3,4],[5,6],[1,2]]]]}","collection":"{\"type\": \"GeometryCollection\", \"geometries\": [{\"type\": \"Point\", \"coordinates\": [1,2]}]}"}}}"###
        );

        Ok(())
    }
}
