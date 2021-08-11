use query_engine_tests::*;

#[test_suite(schema(autoinc_id), capabilities(CreateMany, AutoIncrement))]
mod not_in_batching {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn nested_update_many_timestamps(runner: Runner) -> TestResult<()> {
        runner.query(r#"mutation { createManyTestModel(data: [{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}]) { count }}"#).await?.assert_success();

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyTestModel(where: { id: { notIn: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] } }) { id }}"),
            @r###"{"data":{"findManyTestModel":[{"id":11},{"id":12},{"id":13},{"id":14},{"id":15},{"id":16},{"id":17},{"id":18},{"id":19},{"id":20},{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10},{"id":12},{"id":13},{"id":14},{"id":15},{"id":16},{"id":17},{"id":18},{"id":19},{"id":20}]}}"###
        );

        Ok(())
    }
}
