use query_engine_tests::*;

#[test_suite(schema(schemas::generic))]
mod find_many {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn return_empty(runner: &Runner) -> TestResult<()> {
        assert_query!(
            runner,
            "query { findManyTestModel { id } }",
            r#"{"data":{"findManyTestModel":[]}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn return_all(runner: &Runner) -> TestResult<()> {
        test_data(runner).await?;

        assert_query!(
            runner,
            "query { findManyTestModel { id } }",
            r#"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"#
        );

        Ok(())
    }

    // Todo: Maybe move this "common" test data creation somewhere? (copied from find_first)
    async fn test_data(runner: &Runner) -> TestResult<()> {
        test_row(runner, r#"{ id: 1, field: "test1" }"#).await?;
        test_row(runner, r#"{ id: 2, field: "test2" }"#).await?;
        test_row(runner, r#"{ id: 3 }"#).await?;
        test_row(runner, r#"{ id: 4 }"#).await?;
        test_row(runner, r#"{ id: 5, field: "test3" }"#).await?;

        Ok(())
    }

    async fn test_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
