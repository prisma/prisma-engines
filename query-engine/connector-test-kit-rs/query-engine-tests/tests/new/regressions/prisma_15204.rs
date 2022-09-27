use query_engine_tests::*;

#[test_suite(only(Sqlite))]
mod conversion_error {
    fn schema_int() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                field Int
            }"#
        };

        schema.to_owned()
    }

    fn schema_bigint() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                field BigInt
            }"#
        };

        schema.to_owned()
    }

    async fn test(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            fmt_query_raw(
                r#"INSERT INTO "TestModel" ("id", "field") VALUES (1, 1.84467440724388e+19)"#,
                vec![]
            )
        );

        let res = runner.query(r#"query { findManyTestModel { field } }"#).await?;

        res.assert_failure(
            2020,
            Some("Unable to convert BigDecimal value \"18446744072438800000\" to type i64".into()),
        );

        Ok(())
    }

    #[connector_test(schema(schema_int))]
    async fn convert_to_int(runner: Runner) -> TestResult<()> {
        test(runner).await
    }

    #[connector_test(schema(schema_bigint))]
    async fn convert_to_bigint(runner: Runner) -> TestResult<()> {
        test(runner).await
    }
}
