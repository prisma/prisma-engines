use query_engine_tests::*;

/// New test to check that SQL Server doesn't allow writing autoincrement IDs.
#[test_suite(schema(autoinc_id), only(SqlServer))]
mod sql_server {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn disallow_sql_server(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createManyTestModel(data: [{ id: 2 }]) { count }}",
            2009,
            "Field does not exist on enclosing type."
        );

        insta::assert_snapshot!(
          run_query!(runner, "mutation { createManyTestModel(data: [{}]) { count }}"),
          @r###"{"data":{"createManyTestModel":{"count":1}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(autoinc_id), capabilities(AutoIncrement))]
mod single_col {
    use query_engine_tests::run_query;

    #[connector_test]
    async fn foo(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, "mutation { createManyTestModel(data: [{},{}]) { count }}"),
          @r###"{"data":{"createManyTestModel":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, "query { findManyTestModel { id }}"),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }
}
