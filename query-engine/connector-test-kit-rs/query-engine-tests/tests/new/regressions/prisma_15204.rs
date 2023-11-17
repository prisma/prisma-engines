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

    #[connector_test(schema(schema_int), only(Sqlite), exclude(JS))]
    async fn convert_to_int_sqlite_quaint(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"query { findManyTestModel { field } }"#,
            2023,
            "Inconsistent column data: Could not convert from `BigDecimal(18446744072438800000)` to `Int`"
        );

        Ok(())
    }

    #[connector_test(schema(schema_int), only(Sqlite, JS))]
    async fn convert_to_int_sqlite_js(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"query { findManyTestModel { field } }"#,
            2023,
            "Inconsistent column data: Conversion failed: number must be an integer in column 'field', got '1.84467440724388e19'"
        );

        Ok(())
    }

    #[connector_test(schema(schema_bigint), only(Sqlite), exclude(JS))]
    async fn convert_to_bigint_sqlite_quaint(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"query { findManyTestModel { field } }"#,
            2023,
            "Inconsistent column data: Could not convert from `BigDecimal(18446744072438800000)` to `BigInt`"
        );

        Ok(())
    }

    #[connector_test(schema(schema_bigint), only(Sqlite, JS))]
    async fn convert_to_bigint_sqlite_js(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"query { findManyTestModel { field } }"#,
            2023,
            "Inconsistent column data: Conversion failed: number must be an integer in column 'field', got '1.84467440724388e19'"
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            fmt_query_raw(
                r#"INSERT INTO "TestModel" ("id", "field") VALUES (1, 1.84467440724388e+19)"#,
                vec![]
            )
        );

        Ok(())
    }
}
