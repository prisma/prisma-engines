use query_engine_tests::*;

#[test_suite(schema(autoinc_id), capabilities(CreateMany, AutoIncrement), exclude(CockroachDb))]
mod not_in_batching {
    use query_engine_tests::Runner;

    #[connector_test(exclude(
        CockroachDb,
        Postgres("pg.js.wasm"),
        Postgres("neon.js.wasm"),
        Sqlite("libsql.js.wasm"),
        Vitess("planetscale.js.wasm")
    ))]
    async fn not_in_batch_filter(runner: Runner) -> TestResult<()> {
        runner.query(r#"mutation { createManyTestModel(data: [{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}]) { count }}"#).await?.assert_success();

        assert_error!(
            runner,
            "query { findManyTestModel(where: { id: { notIn: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] } }) { id }}",
            2029 // QueryParameterLimitExceeded
        );

        Ok(())
    }
}

#[test_suite(schema(autoinc_id_cockroachdb), only(CockroachDb))]
mod not_in_batching_cockroachdb {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn not_in_batch_filter(runner: Runner) -> TestResult<()> {
        runner.query(r#"mutation { createManyTestModel(data: [{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}]) { count }}"#).await?.assert_success();

        assert_error!(
            runner,
            "query { findManyTestModel(where: { id: { notIn: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] } }) { id }}",
            2029 // QueryParameterLimitExceeded
        );

        Ok(())
    }
}
