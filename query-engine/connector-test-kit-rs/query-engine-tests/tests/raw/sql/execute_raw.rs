use query_engine_tests::*;

#[test_suite(schema(common_nullable_types), only(Postgres))]
mod postgres_exec_raw {
    // Checks that query raw inputs are correctly coerced to the correct types
    #[connector_test]
    async fn scalar_input_correctly_coerced(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "string", "int", "bInt", "float", "bytes", "bool", "dt") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);"#,
                vec![
                    RawValue::scalar(1_i32),
                    RawValue::scalar("str"),
                    RawValue::scalar(42_i32),
                    RawValue::bigint(9223372036854775807),
                    RawValue::scalar(1.5432_f64),
                    RawValue::bytes(&[1, 2, 3]),
                    RawValue::scalar(true),
                    RawValue::try_datetime("1900-10-10T01:10:10.001Z")?
                ],
            )
        );

        insta::assert_snapshot!(
          run_query!(&runner, "{ findManyTestModel { id, string, int, bInt, float, bool, dt } }"),
          @r###"{"data":{"findManyTestModel":[{"id":1,"string":"str","int":42,"bInt":"9223372036854775807","float":1.5432,"bool":true,"dt":"1999-05-01T00:00:00.000Z"}]}}"###
        );

        Ok(())
    }

    fn decimal() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                decimal Decimal?
            }"#
        };

        schema.to_owned()
    }

    // Checks that query raw inputs are correctly coerced to the correct types
    #[connector_test(schema(decimal))]
    async fn decimal_input_correctly_coerced(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "decimal") VALUES ($1, $2);"#,
                vec![RawValue::scalar(1_i32), RawValue::decimal("123.456789")],
            )
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel { id decimal } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"decimal":"123.456789"}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
