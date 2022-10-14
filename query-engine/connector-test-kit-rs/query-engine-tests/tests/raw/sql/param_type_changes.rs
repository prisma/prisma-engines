use query_engine_tests::*;

#[test_suite(only(Postgres))]
mod param_type_changes {
    #[connector_test(schema(common_numeric_types))]
    async fn null_scalar_lists(runner: Runner) -> TestResult<()> {
        let sql = r#""#;
        let params_1 = todo!();
        let params_2 = todo!();
    }
}
