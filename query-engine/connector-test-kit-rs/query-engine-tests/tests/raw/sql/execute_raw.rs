use query_engine_tests::*;

#[test_suite(schema(common_nullable_types), only(Postgres))]
mod postgres_exec_raw {
    use query_engine_tests::*;
    use serde_json::{json, Value};

    // Checks that query raw inputs are correctly coerced to the correct types
    #[connector_test]
    async fn scalar_input_correctly_coerced(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            execute_raw(
                r#"INSERT INTO "TestModel" ("id", "string", "int", "bInt", "float", "bytes", "bool", "dt") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);"#,
                vec![
                    Value::from(1),
                    Value::from("str"),
                    Value::from(42),
                    scalar_type("bigint", "9223372036854775807"),
                    Value::from(1.5432_f64),
                    scalar_type("bytes", encode_bytes(&[1, 2, 3])),
                    Value::from(true),
                    scalar_type("date", "1999-05-01T00:00:00.000Z")
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
            execute_raw(
                r#"INSERT INTO "TestModel" ("id", "decimal") VALUES ($1, $2);"#,
                vec![Value::from(1), scalar_type("decimal", "123.456789")],
            )
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel { id decimal } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"decimal":"123.456789"}]}}"###
        );

        Ok(())
    }

    fn execute_raw(query: &str, params: Vec<serde_json::Value>) -> String {
        let params = serde_json::to_string(&params).unwrap();

        format!(
            r#"mutation {{ executeRaw(query: "{}", parameters: "{}") }}"#,
            query.replace('"', "\\\""),
            params.replace('"', "\\\"")
        )
    }

    fn query_raw(query: &str, params: Vec<serde_json::Value>) -> String {
        let params = serde_json::to_string(&params).unwrap();

        format!(
            r#"mutation {{ queryRaw(query: "{}", parameters: "{}") }}"#,
            query.replace('"', "\\\""),
            params.replace('"', "\\\"")
        )
    }

    fn scalar_type(type_name: &str, value: impl Into<serde_json::Value>) -> serde_json::Value {
        let value: Value = value.into();

        json!({ "prisma__type": type_name, "prisma__value": value })
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
