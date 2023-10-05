use query_engine_tests::*;

#[test_suite(schema(generic), exclude(MongoDb))]
mod prisma_21369 {
    #[connector_test]
    async fn select_null_works(runner: Runner) -> TestResult<()> {
        let query = fmt_query_raw("SELECT null as result", []);
        let result = run_query!(runner, query);

        assert_eq!(
            result,
            r#"{"data":{"queryRaw":[{"result":{"prisma__type":"null","prisma__value":null}}]}}"#
        );

        Ok(())
    }
}
