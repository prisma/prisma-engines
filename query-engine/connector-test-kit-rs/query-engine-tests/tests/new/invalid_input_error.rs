use query_engine_tests::*;

#[test_suite(schema(generic), only(Postgres))]
mod find_many {
    #[connector_test]
    async fn value_too_large_to_transmit(runner: Runner) -> TestResult<()> {
        let n = 32767;

        // [1,2,...,n]
        let ids: Vec<u32> = (1..n + 1).collect();

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
    #[connector_test]
    async fn value_too_large_to_transmit(runner: Runner) -> TestResult<()> {
        let n = 32768;

        // [1,2,...,n]
        let ids: Vec<u32> = (1..n + 1).collect();

        // "$1,$2,...,$n"
        let params: String = ids
            .iter()
            .map(|id| format!("${}", id))
            .collect::<Vec<String>>()
            .join(",");

        let mutation = format!(
            r#"
            mutation {{
              queryRaw(
                query: "SELECT * FROM \"TestModel\" WHERE id IN ({})",
                parameters: "{:?}"
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
