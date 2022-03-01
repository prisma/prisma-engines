use query_engine_tests::*;

#[test_suite(schema(schemas::user_posts))]
mod distinct {
    use indoc::indoc;
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn empty_database(runner: Runner) -> TestResult<()> {
        assert_query!(
            runner,
            "query { findManyUser(distinct: [first_name, last_name]) { id, first_name, last_name } }",
            r#"{"data":{"findManyUser":[]}}"#
        );

        Ok(())
    }

    /// Regression test for not selecting the fields the distinct is performed on: https://github.com/prisma/prisma/issues/5969
    #[connector_test]
    async fn no_panic(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(&runner, r#"{ id: 2, first_name: "Doe", last_name: "Joe", email: "2" }"#).await?;

        assert_query!(
            runner,
            "query { findManyUser(distinct: [first_name, last_name]) { id } }",
            r#"{"data":{"findManyUser":[{"id":1},{"id":2}]}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn with_duplicates(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        assert_query!(
            runner,
            "query { findManyUser(distinct: [first_name, last_name]) { id, first_name, last_name } }",
            r#"{"data":{"findManyUser":[{"id":1,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn with_skip(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        assert_query!(
            runner,
            "query { findManyUser(skip: 1, distinct: [first_name, last_name]) { id, first_name, last_name } }",
            r#"{"data":{"findManyUser":[{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn with_skip_orderby(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        assert_query!(
            runner,
            "query { findManyUser(orderBy: { first_name: asc }, skip: 1, distinct: [first_name, last_name]) { first_name, last_name } }",
            r#"{"data":{"findManyUser":[{"first_name":"Joe","last_name":"Doe"}]}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn with_skip_orderby_nondistinct(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        assert_query!(
            runner,
            "query { findManyUser(orderBy: { id: desc }, distinct: [first_name, last_name]) { id, first_name, last_name } }",
            r#"{"data":{"findManyUser":[{"id":3,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"#
        );

        Ok(())
    }

    /// Mut return only distinct records for top record, and only for those the distinct relation records.
    #[connector_test]
    async fn nested_distinct(runner: Runner) -> TestResult<()> {
        nested_dataset(&runner).await?;

        // Returns Users 1, 3, 4, 5 top
        // 1 => ["3", "1", "2"]
        // 3 => []
        // 4 => ["1"]
        // 5 => ["2", "3"]
        assert_query!(
            runner,
            indoc! {"{
                findManyUser(distinct: [first_name, last_name]) {
                  id
                  posts(distinct: [title], orderBy: { id: asc }) {
                    title
                  }
                }
              }"},
            r#"{"data":{"findManyUser":[{"id":1,"posts":[{"title":"3"},{"title":"1"},{"title":"2"}]},{"id":3,"posts":[]},{"id":4,"posts":[{"title":"1"}]},{"id":5,"posts":[{"title":"2"},{"title":"3"}]}]}}"#
        );

        Ok(())
    }

    /// Mut return only distinct records for top record, and only for those the distinct relation records. Both orderings reversed.
    #[connector_test]
    async fn nested_distinct_reversed(runner: Runner) -> TestResult<()> {
        nested_dataset(&runner).await?;

        // Returns Users 1, 3, 4, 5 top
        // 5 => ["2", "3"]
        // 4 => ["1"]
        // 3 => []
        // 2 => ["2", "1"]
        assert_query!(
            runner,
            indoc! {"{
                findManyUser(distinct: [first_name, last_name], orderBy: { id: desc }) {
                  id
                  posts(distinct: [title], orderBy: { id: desc }) {
                    title
                  }
                }
              }"},
            r#"{"data":{"findManyUser":[{"id":5,"posts":[{"title":"2"},{"title":"3"}]},{"id":4,"posts":[{"title":"1"}]},{"id":3,"posts":[]},{"id":2,"posts":[{"title":"2"},{"title":"1"}]}]}}"#
        );

        Ok(())
    }

    /// Dataset:
    /// User (id) => Posts (titles, id asc)
    /// 1 => ["3", "1", "1", "2", "1"]
    /// 2 => ["1", "2"]
    /// 3 => []
    /// 4 => ["1", "1"]
    /// 5 => ["2", "3", "2"]
    async fn nested_dataset(runner: &Runner) -> TestResult<()> {
        test_user(
            runner,
            r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1", posts: {
            create: [
                { id: 1, title: "3" }
                { id: 2, title: "1" }
                { id: 3, title: "1" }
                { id: 4, title: "2" }
                { id: 5, title: "1" }
            ]
        }}"#,
        )
        .await?;

        test_user(
            runner,
            r#"{ id: 2, first_name: "Joe", last_name: "Doe", email: "2", posts: {
            create: [
                { id: 6, title: "1" }
                { id: 7, title: "2" }
            ]
        }}"#,
        )
        .await?;

        test_user(
            runner,
            r#"{ id: 3, first_name: "Rocky", last_name: "Balboa", email: "3" }"#,
        )
        .await?;

        test_user(
            runner,
            r#"{ id: 4, first_name: "Papa", last_name: "Elon", email: "4", posts: {
            create: [
                { id: 8, title: "1" }
                { id: 9, title: "1" }
            ]
        }}"#,
        )
        .await?;

        test_user(
            runner,
            r#"{ id: 5, first_name: "Troll", last_name: "Face", email: "5", posts: {
            create: [
                { id: 10, title: "2" }
                { id: 11, title: "3" }
                { id: 12, title: "2" }
            ]
        }}"#,
        )
        .await?;

        Ok(())
    }

    async fn test_user(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneUser(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }
}
