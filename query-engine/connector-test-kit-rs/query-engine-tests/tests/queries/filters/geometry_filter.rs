use query_engine_tests::*;

#[test_suite(capabilities(Geometry))]
mod geometry_filter_spec {
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model TestModel {
                #id(id, Int, @id)
                geom Geometry?
              }
            "#
        };

        schema.to_owned()
    }

    fn schema_postgres() -> String {
        let schema = indoc! {
            r#"
            model TestModel {
              @@schema("test")
              #id(id, Int, @id)
              geom Geometry?
            }
          "#
        };

        schema.to_owned()
    }

    async fn basic_where_test(runner: Runner) -> TestResult<()> {
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

    async fn where_shorthands_test(runner: Runner) -> TestResult<()> {
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

    async fn geometric_comparison_filters_test(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // geoWithin
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { geoWithin: "{\"type\":\"Polygon\",\"coordinates\":[[[1,1],[1,4],[4,4],[4,1],[1,1]]]}" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not geoWithin
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: [{ geom: { not: { geoWithin: "{\"type\":\"Polygon\",\"coordinates\":[[[1,1],[1,4],[4,4],[4,1],[1,1]]]}" }}}, { geom: { not: null }}]}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // geoIntersects
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { geom: { geoIntersects: "{\"type\":\"Polygon\",\"coordinates\":[[[-1,-1],[-1,1],[1,1],[1,-1],[-1,-1]]]}" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // Not geoIntersects
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: [{ geom: { not: { geoIntersects: "{\"type\":\"Polygon\",\"coordinates\":[[[-1,-1],[-1,1],[1,1],[1,-1],[-1,-1]]]}" }}}, { geom: { not: null }}]}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(
        schema(schema),
        exclude(Postgres, MySql(5.6), Sqlite(3, "cfd1", "libsql.js", "libsql.js.wasm"))
    )]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        basic_where_test(runner).await
    }

    #[connector_test(
        schema(schema),
        exclude(Postgres, MySql(5.6), Sqlite(3, "cfd1", "libsql.js", "libsql.js.wasm"))
    )]
    async fn where_shorthands(runner: Runner) -> TestResult<()> {
        where_shorthands_test(runner).await
    }

    // This test should work for MariaDB but doesn't so we skip it for now,
    // see discussion here: https://github.com/prisma/prisma-engines/pull/4208#issuecomment-1828997865
    #[connector_test(
        schema(schema),
        exclude(Postgres, Sqlite(3, "cfd1", "libsql.js", "libsql.js.wasm"), MySQL("mariadb", 5.6)),
        capabilities(GeometryFiltering)
    )]
    async fn geometric_comparison_filters(runner: Runner) -> TestResult<()> {
        geometric_comparison_filters_test(runner).await
    }

    #[connector_test(schema(schema_postgres), db_schemas("public", "test"), only(Postgres("16-postgis")))]
    async fn basic_where_postgres(runner: Runner) -> TestResult<()> {
        basic_where_test(runner).await
    }

    #[connector_test(schema(schema_postgres), db_schemas("public", "test"), only(Postgres("16-postgis")))]
    async fn where_shorthands_postgres(runner: Runner) -> TestResult<()> {
        where_shorthands_test(runner).await
    }

    #[connector_test(schema(schema_postgres), db_schemas("public", "test"), only(Postgres("16-postgis")))]
    async fn geometric_comparison_filters_postgres(runner: Runner) -> TestResult<()> {
        geometric_comparison_filters_test(runner).await
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
