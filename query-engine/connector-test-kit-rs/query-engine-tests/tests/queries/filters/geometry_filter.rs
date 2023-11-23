use query_engine_tests::*;

#[test_suite(
    schema(schema),
    capabilities(GeoJsonGeometry),
    exclude(Postgres(9, 10, 11, 12, 13, 14, 15, "pgbouncer"))
)]
mod geometry_filter_spec {
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model TestModel {
                #id(id, Int, @id)
                geom GeoJson?
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { equals: "{\"type\":\"Point\",\"coordinates\":[0, 0]}" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: [{ geom: { not: "{\"type\":\"Point\",\"coordinates\":[0, 0]}" }}, { geom: { not: null }}] }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { not: null }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn where_shorthands(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: "{\"type\":\"Point\",\"coordinates\":[0, 0]}" }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        match_connector_result!(
          &runner,
          r#"query { findManyTestModel(where: { geom: null }) { id }}"#,
          // MongoDB excludes undefined fields
          MongoDb(_) => vec![r#"{"data":{"findManyTestModel":[]}}"#],
          _ => vec![r#"{"data":{"findManyTestModel":[{"id":3}]}}"#]
        );

        Ok(())
    }

    #[connector_test(capabilities(GeometryFiltering))]
    async fn geometric_comparison_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // geoWithin
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { geoWithin: "{\"type\":\"Polygon\",\"coordinates\":[[[1,1],[1,4],[4,4],[4,1],[1,1]]]}" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not geoWithin
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { not: { geoWithin: "{\"type\":\"Polygon\",\"coordinates\":[[[1,1],[1,4],[4,4],[4,1],[1,1]]]}" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // geoIntersects
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { geoIntersects: "{\"type\":\"Point\",\"coordinates\":[0, 0]}" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // Not geoIntersects
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { not: { geoIntersects: "{\"type\":\"Point\",\"coordinates\":[0, 0]}" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc! { r#"
                mutation { createOneTestModel(data: {
                    id: 1,
                    geom: "{\"type\":\"Point\",\"coordinates\":[0, 0]}",
                }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation { createOneTestModel(data: {
                    id: 2,
                    geom: "{\"type\":\"Polygon\",\"coordinates\":[[[2,2],[2,3],[3,3],[3,2],[2,2]]]}",
                }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
            .await?
            .assert_success();

        Ok(())
    }
}
