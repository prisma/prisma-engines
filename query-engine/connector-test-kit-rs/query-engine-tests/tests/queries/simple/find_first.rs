use query_engine_tests::prelude::*;

#[test_suite]
mod find_first_query {
    #[connector_test(schema(schemas::basic))]
    async fn fetch_first_matching(runner: &Runner) -> TestResult<()> {
        test_data(runner).await?;

        let result = runner
            .query("query { findFirstTestModel(where: { id: 1 }) { id }}")
            .await?;

        assert_eq!(result.to_string(), r#"{"data":{"findFirstTestModel":{"id":1}}}"#);

        let result = runner
            .query("query { findFirstTestModel(where: { field: { not: null }}) { id }}")
            .await?;

        assert_eq!(result.to_string(), r#"{"data":{"findFirstTestModel":{"id":1}}}"#);

        let result = runner
            .query("query { findFirstTestModel(where: { field: { not: null }}, orderBy: { id: desc }) { id }}")
            .await?;

        assert_eq!(result.to_string(), r#"{"data":{"findFirstTestModel":{"id":5}}}"#);

        let result = runner
            .query("query { findFirstTestModel(where: { field: { not: null }}, cursor: { id: 1 }, take: 1, skip: 1, orderBy: { id: asc }) { id }}")
            .await?;

        assert_eq!(result.to_string(), r#"{"data":{"findFirstTestModel":{"id":2}}}"#);
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
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
