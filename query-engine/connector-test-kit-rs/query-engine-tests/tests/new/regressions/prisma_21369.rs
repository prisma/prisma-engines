use query_engine_tests::*;

#[test_suite(schema(generic), exclude(MongoDb))]
mod prisma_21369 {
    #[connector_test]
    async fn select_null_works(runner: Runner) -> TestResult<()> {
        match_connector_result!(
            &runner,
            fmt_query_raw("SELECT NULL AS result", []),
            Sqlite(_) | MySql(_) | SqlServer(_) | Vitess(_) => vec![r#"{"data":{"queryRaw":{"columns":["result"],"types":["int"],"rows":[[null]]}}}"#],
            _ => vec![r#"{"data":{"queryRaw":{"columns":["result"],"types":["string"],"rows":[[null]]}}}"#]

        );

        Ok(())
    }
}
