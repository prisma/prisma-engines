use query_engine_tests::*;

#[test_suite(schema(schemas::user))]
mod find_unique {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn fetch_unique_by_id(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            runner,
            "query { findUniqueUser(where: { id: 1 }) { id } }",
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn fetch_unique_by_single_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { email: "a@b.com" }) { id } }"#,
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn fetch_unique_by_multi_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { first_name_last_name: { first_name: "Elongated", last_name: "Muskrat" } }) { id } }"#,
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_fetch_unique_by_id(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            runner,
            "query { findUniqueUser(where: { id: 2 }) { id } }",
            r#"{"data":{"findUniqueUser":null}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_fetch_unique_by_single_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { email: "b@a.com" }) { id } }"#,
            r#"{"data":{"findUniqueUser":null}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_fetch_unique_by_multi_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { first_name_last_name: { first_name: "Doesn't", last_name: "Exist" } }) { id } }"#,
            r#"{"data":{"findUniqueUser":null}}"#
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
