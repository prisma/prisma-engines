use query_engine_tests::*;

#[test_suite(schema(schemas::user))]
mod find_unique_or_throw {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn find_unique_or_throw_when_record_is_found(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUserOrThrow(where: { email: "a@b.com" }) { id } }"#,
            r#"{"data":{"findUniqueUserOrThrow":{"id":1}}}"#
        );

        assert_query!(
            &runner,
            r#"query { findUniqueUserOrThrow(where: { first_name_last_name: { first_name: "Elongated", last_name: "Muskrat" } }) { id } }"#,
            r#"{"data":{"findUniqueUserOrThrow":{"id":1}}}"#
        );

        assert_query!(
            runner,
            "query { findUniqueUserOrThrow(where: { id: 1 }) { id } }",
            r#"{"data":{"findUniqueUserOrThrow":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_find_unique_by_id(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_error!(
            &runner,
            "query { findUniqueUserOrThrow(where: { id: 2 }) { id } }",
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No record was found for a query."
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_find_unique_by_single_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_error!(
            &runner,
            r#"query { findUniqueUserOrThrow(where: { email: "b@a.com" }) { id } }"#,
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No record was found for a query."
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_find_unique_by_multi_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_error!(
            &runner,
            r#"query { findUniqueUserOrThrow(where: { first_name_last_name: { first_name: "Doesn't", last_name: "Exist" } }) { id } }"#,
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No record was found for a query."
        );

        Ok(())
    }

    async fn test_user(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneUser(data: { id: 1, email: "a@b.com", first_name: "Elongated", last_name: "Muskrat" }) { id } }"#)
            .await?.assert_success();

        Ok(())
    }
}
