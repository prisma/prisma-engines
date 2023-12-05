use query_engine_tests::*;

#[test_suite(only(Sqlite("3-spatialite")))]
mod sqlite {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_ewkt_geometry() -> String {
        let schema = indoc! {
            r#"model Model {
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

    // "Spatialite common geometry types" should "work"
    #[connector_test(schema(schema_ewkt_geometry))]
    async fn native_ewkt_geometry(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              geometry: "POINT(1 2)"
              geometry_point: "POINT(1 2)"
              geometry_line: "LINESTRING(1 2,3 4)"
              geometry_poly: "POLYGON((1 2,3 4,5 6,1 2))"
              geometry_multipoint: "MULTIPOINT(1 2)"
              geometry_multiline: "MULTILINESTRING((1 2,3 4))"
              geometry_multipoly: "MULTIPOLYGON(((1 2,3 4,5 6,1 2)))"
              geometry_collection: "GEOMETRYCOLLECTION(POINT(1 2))"
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
            @r###"{"data":{"createOneModel":{"geometry":"POINT(1 2)","geometry_point":"POINT(1 2)","geometry_line":"LINESTRING(1 2,3 4)","geometry_poly":"POLYGON((1 2,3 4,5 6,1 2))","geometry_multipoint":"MULTIPOINT(1 2)","geometry_multiline":"MULTILINESTRING((1 2,3 4))","geometry_multipoly":"MULTIPOLYGON(((1 2,3 4,5 6,1 2)))","geometry_collection":"GEOMETRYCOLLECTION(POINT(1 2))"}}}"###
        );

        Ok(())
    }

    fn schema_ewkt_geometry_srid() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(id, String, @id, @default(cuid()))
            geometry             Geometry @test.Geometry(Geometry, 4326)
            geometry_point       Geometry @test.Geometry(Point, 4326)
            geometry_line        Geometry @test.Geometry(LineString, 4326)
            geometry_poly        Geometry @test.Geometry(Polygon, 4326)
            geometry_multipoint  Geometry @test.Geometry(MultiPoint, 4326)
            geometry_multiline   Geometry @test.Geometry(MultiLineString, 4326)
            geometry_multipoly   Geometry @test.Geometry(MultiPolygon, 4326)
            geometry_collection  Geometry @test.Geometry(GeometryCollection, 4326)
          }"#
        };

        schema.to_owned()
    }

    // "Spatialite common geometry types with srid" should "work"
    #[connector_test(schema(schema_ewkt_geometry_srid))]
    async fn native_geometry_srid(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              geometry: "SRID=4326;POINT(1 2)"
              geometry_point: "SRID=4326;POINT(1 2)"
              geometry_line: "SRID=4326;LINESTRING(1 2,3 4)"
              geometry_poly: "SRID=4326;POLYGON((1 2,3 4,5 6,1 2))"
              geometry_multipoint: "SRID=4326;MULTIPOINT(1 2)"
              geometry_multiline: "SRID=4326;MULTILINESTRING((1 2,3 4))"
              geometry_multipoly: "SRID=4326;MULTIPOLYGON(((1 2,3 4,5 6,1 2)))"
              geometry_collection: "SRID=4326;GEOMETRYCOLLECTION(POINT(1 2))"
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
            @r###"{"data":{"createOneModel":{"geometry":"SRID=4326;POINT(1 2)","geometry_point":"SRID=4326;POINT(1 2)","geometry_line":"SRID=4326;LINESTRING(1 2,3 4)","geometry_poly":"SRID=4326;POLYGON((1 2,3 4,5 6,1 2))","geometry_multipoint":"SRID=4326;MULTIPOINT(1 2)","geometry_multiline":"SRID=4326;MULTILINESTRING((1 2,3 4))","geometry_multipoly":"SRID=4326;MULTIPOLYGON(((1 2,3 4,5 6,1 2)))","geometry_collection":"SRID=4326;GEOMETRYCOLLECTION(POINT(1 2))"}}}"###
        );

        Ok(())
    }

    fn schema_geojson_geometry() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(id, String, @id, @default(cuid()))
            geometry             GeoJson @test.Geometry(Geometry, 4326)
            geometry_point       GeoJson @test.Geometry(Point, 4326)
            geometry_line        GeoJson @test.Geometry(LineString, 4326)
            geometry_poly        GeoJson @test.Geometry(Polygon, 4326)
            geometry_multipoint  GeoJson @test.Geometry(MultiPoint, 4326)
            geometry_multiline   GeoJson @test.Geometry(MultiLineString, 4326)
            geometry_multipoly   GeoJson @test.Geometry(MultiPolygon, 4326)
            geometry_collection  GeoJson @test.Geometry(GeometryCollection, 4326)
          }"#
        };

        schema.to_owned()
    }

    // "Spatialite geometry types" should "work" with GeoJSON
    #[connector_test(schema(schema_geojson_geometry))]
    async fn native_geojson_geometry(runner: Runner) -> TestResult<()> {
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
        @r###"{"data":{"createOneModel":{"geometry":"{\"type\": \"Point\", \"coordinates\": [1,2]}","geometry_point":"{\"type\": \"Point\", \"coordinates\": [1,2]}","geometry_line":"{\"type\": \"LineString\", \"coordinates\": [[1,2],[3,4]]}","geometry_poly":"{\"type\": \"Polygon\", \"coordinates\": [[[1,2],[3,4],[5,6],[1,2]]]}","geometry_multipoint":"{\"type\": \"MultiPoint\", \"coordinates\": [[1,2]]}","geometry_multiline":"{\"type\": \"MultiLineString\", \"coordinates\": [[[1,2],[3,4]]]}","geometry_multipoly":"{\"type\": \"MultiPolygon\", \"coordinates\": [[[[1,2],[3,4],[5,6],[1,2]]]]}","geometry_collection":"{\"type\": \"GeometryCollection\", \"geometries\": [{\"type\": \"Point\", \"coordinates\": [1,2]}]}"}}}"###
        );

        Ok(())
    }
}
