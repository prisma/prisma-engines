use query_engine_tests::*;

#[test_suite(schema(generic), exclude(MongoDb))]
mod prisma_21369 {
    #[connector_test]
    async fn select_null_works(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(runner, fmt_query_raw("SELECT NULL AS result", [])),
            @r###"{"data":{"queryRaw":{"columns":["result"],"types":["string"],"rows":[[null]]}}}"###
        );

        Ok(())
    }
}
