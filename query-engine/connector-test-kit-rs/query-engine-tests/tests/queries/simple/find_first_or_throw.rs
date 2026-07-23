use query_engine_tests::*;

#[test_suite(schema(schemas::generic))]
mod find_first_or_throw_query {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn find_first_or_throw_matching(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        assert_query!(
            runner,
            "query { findFirstTestModelOrThrow(where: { id: 1 }) { id }}",
            r#"{"data":{"findFirstTestModelOrThrow":{"id":1}}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModelOrThrow(where: { field: { not: null }}) { id }}",
            r#"{"data":{"findFirstTestModelOrThrow":{"id":1}}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModelOrThrow(where: { field: { not: null }}, orderBy: { id: desc }) { id }}",
            r#"{"data":{"findFirstTestModelOrThrow":{"id":5}}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModelOrThrow(where: { field: { not: null }}, cursor: { id: 1 }, take: 1, skip: 1, orderBy: { id: asc }) { id }}",
            r#"{"data":{"findFirstTestModelOrThrow":{"id":2}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn find_first_or_throw_not_matching(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        assert_error!(
            &runner,
            "query { findFirstTestModelOrThrow(where: { id: 6 }) { id }}",
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No record was found for a query."
        );

        Ok(())
    }

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
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
