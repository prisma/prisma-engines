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

        let old_query_batch_size = std::env::var("QUERY_BATCH_SIZE");
        std::env::set_var("QUERY_BATCH_SIZE", n.to_string());
        assert_error!(
            runner,
            query,
            2034,
            "Assertion violation on the database: `value too large to transmit`"
        );
        std::env::set_var("QUERY_BATCH_SIZE", old_query_batch_size.unwrap_or_default());

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
