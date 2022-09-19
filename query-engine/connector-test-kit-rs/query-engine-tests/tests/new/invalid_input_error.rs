use query_engine_tests::*;

#[test_suite(schema(generic), only(Postgres))]
mod find_many {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn return_assertion_violation_value_too_large_to_transmit(runner: Runner) -> TestResult<()> {
        let n = 32767;
        let ids: Vec<u32> = (1..n).collect();

        let query = format!(
            r#"
            query {{
              findManyTestModel(where: {{
                id: {{
                  in: {:?}
                }}
              }}) {{
                id
              }}
            }}"#,
            ids,
        );

        assert_error!(
            runner,
            query,
            2034,
            "Assertion violation on the database: `value too large to transmit`"
        );

        Ok(())
    }
}

#[test_suite(schema(generic), only(Postgres))]
mod raw_params {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn return_assertion_violation_value_too_large_to_transmit(runner: Runner) -> TestResult<()> {
        let n = 32768;
        let ids: Vec<u32> = (1..n).collect();
        let params: String = ids
            .iter()
            .map(|id| format!("${}", id))
            .collect::<Vec<String>>()
            .join(",");

        let mutation = format!(
            r#"
            mutation {{
              queryRaw(
                query: "SELECT * FROM "public"."TestModel" WHERE id IN ({})",
                parameters: "{:?}"
              ) {{
                json
              }}
              )
            }}"#,
            params, ids,
        );

        assert_error!(
            runner,
            mutation,
            2034,
            "Assertion violation on the database: `value too large to transmit`"
        );

        Ok(())
    }
}
