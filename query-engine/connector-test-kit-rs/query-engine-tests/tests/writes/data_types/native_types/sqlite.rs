use query_engine_tests::*;

#[test_suite(only(Sqlite("3-spatialite")))]
mod sqlite {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_geometry() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(id, String, @id, @default(cuid()))
            geometry            Geometry @test.Geometry(Geometry, 4326)
            geometry_point      Geometry @test.Geometry(Point, 4326)
            geometry_line       Geometry @test.Geometry(LineString, 4326)
            geometry_poly       Geometry @test.Geometry(Polygon, 4326)
            geometry_multipoint Geometry @test.Geometry(MultiPoint, 4326)
            geometry_multiline  Geometry @test.Geometry(MultiLineString, 4326)
            geometry_multipoly  Geometry @test.Geometry(MultiPolygon, 4326)
            geometry_collection Geometry @test.Geometry(GeometryCollection, 4326)
          }"#
        };

        schema.to_owned()
    }

    // "Spatialite geometry types" should "work"
    #[connector_test(schema(schema_geometry))]
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
        @r###"{"data":{"createOneModel":{"geometry":"{\"type\": \"Point\", \"coordinates\": [1,2]}","geometry_point":"{\"type\": \"Point\", \"coordinates\": [1,2]}","geometry_line":"{\"type\": \"LineString\", \"coordinates\": [[1,2],[3,4]]}","geometry_poly":"{\"type\": \"Polygon\", \"coordinates\": [[[1,2],[3,4],[5,6],[1,2]]]}","geometry_multipoint":"{\"type\": \"MultiPoint\", \"coordinates\": [[1,2]]}","geometry_multiline":"{\"type\": \"MultiLineString\", \"coordinates\": [[[1,2],[3,4]]]}","geometry_multipoly":"{\"type\": \"MultiPolygon\", \"coordinates\": [[[[1,2],[3,4],[5,6],[1,2]]]]}","geometry_collection":"{\"type\": \"GeometryCollection\", \"geometries\": [{\"type\": \"Point\", \"coordinates\": [1,2]}]}"}}}"###
        );

        Ok(())
    }

    fn schema_geometry_srid() -> String {
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
    #[connector_test(schema(schema_geometry_srid))]
    async fn native_geometry_srid(runner: Runner) -> TestResult<()> {
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
