use query_engine_tests::*;

#[test_suite(schema(schemas::posts_categories))]
mod m2m {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn fetch_only_associated(runner: &Runner) -> TestResult<()> {
        test_data(runner).await?;

        // Querying categories for one post only return their categories.
        assert_query!(
            runner,
            "query { findUniquePost(where: { id: 1 }) { categories { id }}}",
            r#"{"data":{"findUniquePost":{"categories":[{"id":1},{"id":2}]}}}"#
        );

        // Querying the other way around works the same (2 connected posts here).
        assert_query!(
            runner,
            "query { findUniqueCategory(where: { id: 1 }) { posts { id }}}",
            r#"{"data":{"findUniqueCategory":{"posts":[{"id":1},{"id":2}]}}}"#
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
            createOnePost(data: {
                id: 1,
                title: "Why Prisma is not an ORM",
                content: "Long winded explanation.",
                categories: {
                    create: [
                        {
                            id: 1,
                            name: "Marketing"
                        },
                        {
                            id: 2,
                            name: "Fiction"
                        }
                    ]
                }
            }) { id }
        }"#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation {
            createOnePost(data: {
                id: 2,
                title: "Actually, Prisma is a _modern_ ORM!",
                content: "Explanation why we weren't wrong, while being wrong.",
                categories: {
                    connect: [
                        {
                            id: 1
                        }
                    ]
                }
            }) { id }
        }"#,
            )
            .await?
            .assert_success();

        Ok(())
    }
}
