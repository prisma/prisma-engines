use query_engine_tests::*;

/// It's possible for a to-many disconnect to specify a list of selectors to disconnect.
/// Now, if the inlining is done on the many side (which is usually the case for 1:m), then Prisma MUST ensure
/// that we're only disconnecting records that were previously connected to the parent.
#[test_suite]
mod disconnect_security {
    use query_engine_tests::assert_query;

    #[connector_test(schema(schemas::a1_to_bm_opt), exclude(Sqlite("cfd1")))]
    async fn must_honor_connect_scope_one2m(runner: Runner) -> TestResult<()> {
        one_to_many_test_data(&runner).await?;

        // Diconnecting B3 from A1 (not connected) must NOT disconnect it from A2.
        assert_query! {
            &runner,
            r#"mutation {
                updateOneA(where: { id: 1 }, data: { many_b: { disconnect: { id: 3 } } }) {
                  many_b { id }
                }
              }"#,
            r#"{"data":{"updateOneA":{"many_b":[{"id":1},{"id":2}]}}}"#
        }

        assert_query! {
            &runner,
            r#"query {
                findUniqueA(where: { id: 2 }) {
                    many_b { id }
                }
            }"#,
            r#"{"data":{"findUniqueA":{"many_b":[{"id":3}]}}}"#
        }

        Ok(())
    }

    #[connector_test(schema(schemas::posts_categories), exclude(Sqlite("cfd1")))]
    async fn must_honor_connect_scope_m2m(runner: Runner) -> TestResult<()> {
        many_to_many_test_data(&runner).await?;

        // Diconnecting Category3 from Post1 (not connected) must NOT disconnect it from Post2.
        assert_query! {
            &runner,
            r#"mutation {
                updateOnePost(where: { id: 1 }, data: { categories: { disconnect: { id: 3 } } }) {
                  categories { id }
                }
              }"#,
            r#"{"data":{"updateOnePost":{"categories":[{"id":1},{"id":2}]}}}"#
        }

        assert_query! {
            &runner,
            r#"query {
                findUniquePost(where: { id: 2 }) {
                    categories { id }
                }
            }"#,
            r#"{"data":{"findUniquePost":{"categories":[{"id":3}]}}}"#
        }

        Ok(())
    }

    /// Create test data where:
    /// A1 -> B1, B2
    /// A2 -> B3
    async fn one_to_many_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query("mutation { createOneA(data: { id: 1, many_b: { create: [{ id: 1 }, { id: 2 }] } }) { id }}")
            .await?
            .assert_success();

        runner
            .query("mutation { createOneA(data: { id: 2, many_b: { create: [{ id: 3 }] } }) { id }}")
            .await?
            .assert_success();

        Ok(())
    }

    /// Create test data where:
    /// Post1 -> Category1, Category2
    /// Post2 -> Category3
    async fn many_to_many_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOnePost(data: { id: 1, title: "P1", categories: { create: [{ id: 1, name: "C1" }, { id: 2, name: "C2" }] } }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOnePost(data: { id: 2, title: "P2", categories: { create: [{ id: 3, name: "C3" }] } }) { id }}"#)
            .await?
            .assert_success();

        Ok(())
    }
}
