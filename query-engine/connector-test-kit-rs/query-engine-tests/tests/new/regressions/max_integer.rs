use query_engine_tests::*;

#[test_suite(schema(schema))]
mod max_integer {
    fn schema() -> String {
        let schema = indoc! {r#"
        model Test {
            id  Int @id
            int Int
        }
        "#};

        schema.to_string()
    }

    #[connector_test]
    async fn panics_gql_parser(runner: Runner) -> TestResult<()> {
        runner
            .query(format!(
                "mutation {{ createOneTest(data: {{ id: 1, int: {} }}) {{ id int }} }}",
                "100000000000000000000"
            ))
            .await?
            .assert_success();

        Ok(())
    }

    #[connector_test]
    async fn panics_250_87(runner: Runner) -> TestResult<()> {
        runner
            .query(format!(
                "mutation {{ createOneTest(data: {{ id: 1, int: {} }}) {{ id int }} }}",
                "1e22"
            ))
            .await?
            .assert_success();

        Ok(())
    }
}
