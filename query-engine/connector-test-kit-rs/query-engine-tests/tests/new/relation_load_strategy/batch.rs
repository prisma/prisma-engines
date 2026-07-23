use query_engine_tests::*;

use super::used_db_join_times;

#[test_suite(schema(schema))]
mod relation_load_strategy_batch {
    fn schema() -> String {
        indoc! {
            r#"model User {
                #id(userId, Int, @id)
                email      String    @unique
                posts Post[]
            }

            model Post {
                #id(postId, Int, @id)
                title     String
                authorId  Int
                author    User   @relation(fields: [authorId], references: [userId])
            }"#
        }
        .to_owned()
    }

    async fn create_test_data(runner: &mut Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"
            mutation {
                createOneUser(
                    data: {
                        userId: 1,
                        email: "alice@prisma.io",
                        posts: { create: { postId: 1, title: "Hello world" } }
                    }
                ) {
                    userId
                }
            }
            "#
        );

        run_query!(
            runner,
            r#"
            mutation {
                createOneUser(
                    data: {
                        userId: 2,
                        email: "bob@prisma.io",
                        posts: { create: { postId: 2, title: "Good afternoon world" } }
                    }
                ) {
                    userId
                }
            }
            "#
        );

        // Clean the logs so we do the assertions on the following queries only.
        runner.get_logs().await;

        Ok(())
    }

    #[connector_test(capabilities(LateralJoin))]
    async fn compacted_query_lateral(runner: Runner) -> TestResult<()> {
        compacted_query(runner).await
    }

    #[connector_test(
        capabilities(CorrelatedSubqueries),
        exclude(Mysql("5.6", "5.7", "mariadb", "mariadb.js.wasm"))
    )]
    async fn compacted_query_subquery(runner: Runner) -> TestResult<()> {
        compacted_query(runner).await
    }

    async fn compacted_query(mut runner: Runner) -> TestResult<()> {
        create_test_data(&mut runner).await?;

        let queries = batch([
            r#"query { findUniqueUser(relationLoadStrategy: query, where: { userId: 1 }) { email posts { title } } }"#,
            r#"query { findUniqueUser(relationLoadStrategy: query, where: { userId: 2 }) { email posts { title } } }"#,
        ]);

        let batch_results = runner.batch(queries, false, None).await?;

        insta::assert_snapshot!(
            batch_results.to_string(),
            @r#"{"batchResult":[{"data":{"findUniqueUser":{"email":"alice@prisma.io","posts":[{"title":"Hello world"}]}}},{"data":{"findUniqueUser":{"email":"bob@prisma.io","posts":[{"title":"Good afternoon world"}]}}}]}"#
        );

        let logs = runner.get_logs().await;

        // Two queries in total: a compacted query for both parents and a compacted query for both children.
        assert_eq!(count_queries(&logs), 2);

        // The results of those two queries are joined in memory.
        assert_eq!(used_db_join_times(&logs), 0);

        Ok(())
    }

    #[connector_test(capabilities(LateralJoin))]
    async fn compacted_join_lateral(runner: Runner) -> TestResult<()> {
        compacted_join(runner).await
    }

    #[connector_test(
        capabilities(CorrelatedSubqueries),
        exclude(Mysql("5.6", "5.7", "mariadb", "mariadb.js.wasm"))
    )]
    async fn compacted_join_subquery(runner: Runner) -> TestResult<()> {
        compacted_join(runner).await
    }

    async fn compacted_join(mut runner: Runner) -> TestResult<()> {
        create_test_data(&mut runner).await?;

        let queries = batch([
            r#"query { findUniqueUser(relationLoadStrategy: join, where: { userId: 1 }) { email posts { title } } }"#,
            r#"query { findUniqueUser(relationLoadStrategy: join, where: { userId: 2 }) { email posts { title } } }"#,
        ]);

        let batch_results = runner.batch(queries, false, None).await?;

        insta::assert_snapshot!(
            batch_results.to_string(),
            @r#"{"batchResult":[{"data":{"findUniqueUser":{"email":"alice@prisma.io","posts":[{"title":"Hello world"}]}}},{"data":{"findUniqueUser":{"email":"bob@prisma.io","posts":[{"title":"Good afternoon world"}]}}}]}"#
        );

        let logs = runner.get_logs().await;

        // A single compacted query for both parent queries together with their nested queries.
        assert_eq!(count_queries(&logs), 1);

        // The query uses DB-level join.
        assert_eq!(used_db_join_times(&logs), 1);

        Ok(())
    }

    #[connector_test(capabilities(LateralJoin))]
    async fn mixed_rls_does_not_compact_lateral(runner: Runner) -> TestResult<()> {
        mixed_rls_does_not_compact(runner).await
    }

    #[connector_test(
        capabilities(CorrelatedSubqueries),
        exclude(Mysql("5.6", "5.7", "mariadb", "mariadb.js.wasm"))
    )]
    async fn mixed_rls_does_not_compact_subquery(runner: Runner) -> TestResult<()> {
        mixed_rls_does_not_compact(runner).await
    }

    async fn mixed_rls_does_not_compact(mut runner: Runner) -> TestResult<()> {
        create_test_data(&mut runner).await?;

        let queries = batch([
            r#"query { findUniqueUser(relationLoadStrategy: query, where: { userId: 1 }) { email posts { title } } }"#,
            r#"query { findUniqueUser(relationLoadStrategy: join, where: { userId: 2 }) { email posts { title } } }"#,
        ]);

        let batch_results = runner.batch(queries, false, None).await?;

        insta::assert_snapshot!(
            batch_results.to_string(),
            @r#"{"batchResult":[{"data":{"findUniqueUser":{"email":"alice@prisma.io","posts":[{"title":"Hello world"}]}}},{"data":{"findUniqueUser":{"email":"bob@prisma.io","posts":[{"title":"Good afternoon world"}]}}}]}"#
        );

        let logs = runner.get_logs().await;

        // Two separate queries for the first `findUnique`, one separate joined query for the second `findUnique`.
        assert_eq!(count_queries(&logs), 3);

        // Only the second `findUnique` uses DB-level join.
        assert_eq!(used_db_join_times(&logs), 1);

        Ok(())
    }

    fn batch(queries: impl IntoIterator<Item = &'static str>) -> Vec<String> {
        queries.into_iter().map(ToOwned::to_owned).collect()
    }

    fn count_queries(logs: &[String]) -> usize {
        logs.iter().filter(|l| l.contains("SELECT")).count()
    }
}
