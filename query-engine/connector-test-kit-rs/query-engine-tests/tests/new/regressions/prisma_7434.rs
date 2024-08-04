use query_engine_tests::*;

#[test_suite(schema(autoinc_id), capabilities(AutoIncrement), exclude(CockroachDb))]
mod not_in_chunking {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn not_in_batch_filter(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            with_id_excess!(
                runner,
                "query { findManyTestModel(where: { id: { notIn: [:id_list:] } }) { id }}"
            ),
            2029
        ); // QueryParameterLimitExceeded

        Ok(())
    }
}

#[test_suite(schema(autoinc_id_cockroachdb), only(CockroachDb))]
mod not_in_chunking_cockroachdb {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn not_in_batch_filter(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            with_id_excess!(
                runner,
                "query { findManyTestModel(where: { id: { notIn: [:id_list:] } }) { id }}"
            ),
            2029
        ); // QueryParameterLimitExceeded

        Ok(())
    }
}
