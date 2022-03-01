use query_engine_tests::*;

/// Port note: The `findMany` portion of the old `WhereUniqueSpec` was omitted, didn't add any value.
#[test_suite(schema(schemas::user))]
mod where_unique {
    use query_engine_tests::{assert_error, assert_query};

    #[connector_test]
    async fn no_unique_fields(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            "query { findUniqueUser(where: {}){ id }}",
            2009,
            "Expected exactly one field to be present, got 0."
        );

        Ok(())
    }

    #[connector_test]
    async fn one_unique_field(runner: Runner) -> TestResult<()> {
        test_users(&runner).await?;
        assert_query!(
            &runner,
            "query { findUniqueUser(where: { id: 1 }){ id }}",
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn more_than_one_unique_field(runner: Runner) -> TestResult<()> {
        test_users(&runner).await?;
        assert_error!(
            &runner,
            r#"query { findUniqueUser(where: { id: 1, first_name: "Elongated" }){ id }}"#,
            2009,
            "Field does not exist on enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn implicit_unique_and(runner: Runner) -> TestResult<()> {
        test_users(&runner).await?;
        assert_query!(
            &runner,
            "query { findUniqueUser(where: { id: 1 }){ id }}",
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    async fn test_users(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneUser(data: { id: 1, email: "a@b.com", first_name: "Elongated", last_name: "Muskrat" }) { id } }"#)
            .await?.assert_success();

        runner
            .query(r#"mutation { createOneUser(data: { id: 2, email: "b@a.com", first_name: "John", last_name: "Cena" }) { id } }"#)
            .await?.assert_success();

        Ok(())
    }
}
