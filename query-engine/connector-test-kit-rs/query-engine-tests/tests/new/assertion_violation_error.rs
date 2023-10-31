use query_engine_tests::*;

#[test_suite(schema(generic))]
mod raw_params {
    #[connector_test(only(Postgres), exclude(JS))]
    async fn value_too_many_bind_variables(runner: Runner) -> TestResult<()> {
        let n = 32768;

        // [1,2,...,n]
        let ids: Vec<u32> = (1..n + 1).collect();

        // "$1,$2,...,$n"
        let params: String = ids.iter().map(|id| format!("${id}")).collect::<Vec<String>>().join(",");

        let mutation = format!(
            r#"
            mutation {{
              queryRaw(
                query: "SELECT * FROM \"TestModel\" WHERE id IN ({params})",
                parameters: "{ids:?}"
              )
            }}"#,
        );

        assert_error!(
            runner,
            mutation,
            2035,
            "Assertion violation on the database: `too many bind variables in prepared statement, expected maximum of 32767, received 32768`"
        );

        Ok(())
    }
}
