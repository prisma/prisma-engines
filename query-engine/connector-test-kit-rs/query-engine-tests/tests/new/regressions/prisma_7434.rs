use query_engine_tests::*;

#[test_suite(schema(autoinc_id), capabilities(CreateMany, AutoIncrement))]
mod not_in_batching {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn not_in_batch_filter(runner: Runner) -> TestResult<()> {
        runner.query(r#"mutation { createManyTestModel(data: [{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}]) { count }}"#).await?.assert_success();

        assert_error!(
            runner,
            "query { findManyTestModel(where: { id: { notIn: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] } }) { id }}",
            2029,
            "Your query cannot be split into multiple queries because the filter will cause incorrect results"
        );

        Ok(())
    }
}
