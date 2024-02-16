use query_engine_tests::*;

/// `distinct on` queries involving `orderBy`
/// are currently forced through in-memory handling as otherwise,
/// they fail with the following error message. This does not affect _all_
/// cases of `distinct on` in conjunction with `orderBy`.
///
/// ```sql
/// SELECT DISTINCT ON expressions must match initial ORDER BY expressions
/// ```
///
/// `distinct on` queries _not_ involving `orderBy` return differently ordered
/// result sets, hence we need to duplicate certain tests to track snapshots
/// for both the in-db pg results, and the in-mem result sets.
#[test_suite(schema(schemas::user_posts))]
mod distinct {
    use indoc::indoc;
    use query_engine_tests::{match_connector_result, run_query};

    #[connector_test]
    async fn empty_database(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(
                &runner,
                indoc!("{
                    findManyUser(distinct: [first_name, last_name])
                    { id, first_name, last_name }
                }")
            ),
            @r###"{"data":{"findManyUser":[]}}"###
        );

        Ok(())
    }

    /// Regression test for not selecting the fields the distinct is performed on: https://github.com/prisma/prisma/issues/5969
    #[connector_test]
    async fn no_panic(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(&runner, r#"{ id: 2, first_name: "Doe", last_name: "Joe", email: "2" }"#).await?;

        match_connector_result!(
            &runner,
            indoc!(
                "{
                    findManyUser(distinct: [first_name, last_name])
                    { id }
                }"
            ),
            Postgres(_) => r###"{"data":{"findManyUser":[{"id":2},{"id":1}]}}"###,
            _ => r###"{"data":{"findManyUser":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn shorthand_works(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(&runner, r#"{ id: 2, first_name: "Joe", last_name: "Doe", email: "2" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                &runner,
                indoc!("{
                    findManyUser(distinct: first_name)
                    { id }
                }")
            ),
            @r###"{"data":{"findManyUser":[{"id":1}]}}"###
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

        match_connector_result!(
            &runner,
            indoc!("{
                findManyUser(distinct: [first_name, last_name])
                { id, first_name, last_name }
            }"),
            Postgres(_) => r###"{"data":{"findManyUser":[{"id":2,"first_name":"Hans","last_name":"Wurst"},{"id":1,"first_name":"Joe","last_name":"Doe"}]}}"###,
            _ => r###"{"data":{"findManyUser":[{"id":1,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"###
        );

        Ok(())
    }

    // region: orderBy
    #[connector_test]
    async fn with_orderby_basic(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        insta::assert_snapshot!(run_query!(
                &runner,
                indoc!("{
                    findManyUser(
                        orderBy: { first_name: desc },
                        distinct: [first_name, last_name])
                        { id, first_name, last_name }
                    }")
            ),
            @r###"{"data":{"findManyUser":[{"id":1,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_orderby_non_matching(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        insta::assert_snapshot!(run_query!(
                &runner,
                indoc!("{
                    findManyUser(
                        orderBy: { id: desc },
                        distinct: [first_name, last_name])
                        { id, first_name, last_name }
                    }")
            ),
            @r###"{"data":{"findManyUser":[{"id":3,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_orderby_leftmost_non_matching(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        insta::assert_snapshot!(run_query!(
                &runner,
                indoc!("{
                    findManyUser(
                        orderBy: [{ first_name: desc }, { id: desc }, { last_name: desc }],
                        distinct: [first_name, last_name])
                        { id, first_name, last_name }
                    }")
            ),
            @r###"{"data":{"findManyUser":[{"id":3,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_orderby_similar(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        insta::assert_snapshot!(run_query!(
                &runner,
                indoc!("{
                    findManyUser(
                        orderBy: [{ first_name: desc }, { last_name: desc }, { id: desc }],
                        distinct: [first_name, last_name])
                        { id, first_name, last_name }
                    }")
            ),
            @r###"{"data":{"findManyUser":[{"id":3,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(FullTextSearchWithoutIndex))]
    async fn with_orderby_non_scalar(runner: Runner) -> TestResult<()> {
        test_user(
            &runner,
            r#"{
                id: 1,
                first_name: "Joe",
                last_name: "Doe",
                email: "1"
            }"#,
        )
        .await?;

        test_user(
            &runner,
            r#"{
                id: 2,
                first_name: "Bro",
                last_name: "Doe",
                email: "2"
            }"#,
        )
        .await?;

        match_connector_result!(
            &runner,
            indoc!(
                r#"{
                    findManyUser(
                        orderBy: {
                            _relevance: {
                                fields: ["first_name"],
                                search: "developer",
                                sort: desc 
                            }
                        }
                        distinct: [first_name]
                    )
                    { id, first_name, last_name }
                }"#
            ),
            Postgres(_) => r#"{"data":{"findManyUser":[{"id":1,"first_name":"Joe","last_name":"Doe"},{"id":2,"first_name":"Bro","last_name":"Doe"}]}}"#,
            _ => ""
        );

        Ok(())
    }

    // endregion

    #[connector_test]
    async fn with_skip_basic(runner: Runner) -> TestResult<()> {
        test_user(&runner, r#"{ id: 1, first_name: "Joe", last_name: "Doe", email: "1" }"#).await?;
        test_user(
            &runner,
            r#"{ id: 2, first_name: "Hans", last_name: "Wurst", email: "2" }"#,
        )
        .await?;
        test_user(&runner, r#"{ id: 3, first_name: "Joe", last_name: "Doe", email: "3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                &runner,
                indoc!("{
                    findManyUser(skip: 1, distinct: [first_name, last_name])
                    { id, first_name, last_name }
                }")
            ),
            @r###"{"data":{"findManyUser":[{"id":2,"first_name":"Hans","last_name":"Wurst"}]}}"###
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

        insta::assert_snapshot!(
            run_query!(
                &runner,
                indoc!("{
                    findManyUser(
                        orderBy: { first_name: asc },
                        skip: 1,
                        distinct: [first_name, last_name])
                        { first_name, last_name }
                    }")
            ),
            @r###"{"data":{"findManyUser":[{"first_name":"Joe","last_name":"Doe"}]}}"###
        );

        Ok(())
    }

    /// Mut return only distinct records for top record, and only for those the distinct relation records.
    #[connector_test]
    async fn nested_distinct_id(runner: Runner) -> TestResult<()> {
        nested_dataset(&runner).await?;

        // Returns Users 1, 3, 4, 5 top
        // 1 => ["3", "1", "2"]
        // 4 => ["1"]
        // 3 => []
        // 5 => ["2", "3"]
        insta::assert_snapshot!(
            run_query!(
                &runner,
                indoc!(
                    "{
                findManyUser(distinct: [first_name, last_name], orderBy: {first_name: asc})
                {
                    id
                    posts(distinct: [title], orderBy: { id: asc }) {
                        title
                    }
                }}"
                )
            ),
            @r###"{"data":{"findManyUser":[{"id":1,"posts":[{"title":"3"},{"title":"1"},{"title":"2"}]},{"id":4,"posts":[{"title":"1"}]},{"id":3,"posts":[]},{"id":5,"posts":[{"title":"2"},{"title":"3"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nested_distinct_title(runner: Runner) -> TestResult<()> {
        nested_dataset(&runner).await?;

        // Returns Users 1, 3, 4, 5 top
        // 1 => ["3", "1", "2"]
        // 4 => ["1"]
        // 3 => []
        // 5 => ["2", "3"]

        insta::assert_snapshot!(run_query!(
                &runner,
                indoc!(
                    "{
                findManyUser(
                    distinct: [first_name, last_name], orderBy: {first_name: asc})
                    {
                        id, first_name
                        posts(distinct: [title], orderBy: { title: asc })
                        { title }
                    }
                }")
            ),
            @r###"{"data":{"findManyUser":[{"id":1,"first_name":"Joe","posts":[{"title":"1"},{"title":"2"},{"title":"3"}]},{"id":4,"first_name":"Papa","posts":[{"title":"1"}]},{"id":3,"first_name":"Rocky","posts":[]},{"id":5,"first_name":"Troll","posts":[{"title":"2"},{"title":"3"}]}]}}"###
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
        insta::assert_snapshot!(run_query!(
                &runner,
                indoc! {"{
                    findManyUser(
                        distinct: [first_name, last_name],
                        orderBy: { id: desc }
                    )
                    {
                        id
                        posts(distinct: [title], orderBy: { id: desc }) { title }
                    }
                  }"}
            ),
            @r###"{"data":{"findManyUser":[{"id":5,"posts":[{"title":"2"},{"title":"3"}]},{"id":4,"posts":[{"title":"1"}]},{"id":3,"posts":[]},{"id":2,"posts":[{"title":"2"},{"title":"1"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(posts_categories))]
    async fn m2m_implicit(runner: Runner) -> TestResult<()> {
        m2m_implicit_test_data(&runner).await?;

        insta::assert_snapshot!(run_query!(
                &runner,
                indoc!(r#"{
                    findManyPost(
                        distinct: [title], orderBy: {title: asc})
                        {
                            id, title
                            categories(distinct: [name], orderBy: { name: asc })
                            { id, name }
                        }
                }"#
            )),
            @r###"{"data":{"findManyPost":[{"id":1,"title":"P1","categories":[{"id":1,"name":"C1"},{"id":2,"name":"C2"},{"id":4,"name":"C3"}]},{"id":2,"title":"P2","categories":[{"id":5,"name":"C3"}]}]}}"###
        );

        Ok(())
    }

    async fn m2m_implicit_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createManyCategory(
                        data: [
                            { id: 1, name: "C1" },
                            { id: 2, name: "C2" },
                            { id: 3, name: "C2" },
                            { id: 4, name: "C3" },
                            { id: 5, name: "C3" },
                            { id: 6, name: "C3" }
                        ]
                    )
                    { count }
                }
            "#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation { createOnePost(
                    data: {
                        id: 1, title: "P1", categories: {
                            connect: [{ id: 1 }, { id: 2 }, { id: 3 }, { id: 4 }, { id: 6 }]
                        }
                    })
                    { id }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation { createOnePost(
                    data: {
                        id: 2, title: "P2", categories: {
                            connect: [{ id: 5 }, { id: 6 }]
                        }
                    })
                    { id }
                }"#,
            )
            .await?
            .assert_success();

        Ok(())
    }

    #[connector_test(schema(posts_on_categories))]
    async fn m2m_explicit(runner: Runner) -> TestResult<()> {
        m2m_explicit_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
            query {
                findManyCategoriesOnPosts(
                    orderBy: [{ postId: asc }, { categoryId: asc }],
                    where: {postId: {gt: 0}}
                )
                { category { name }, post { title } }
            }"#),
          @r###""###
        );

        Ok(())
    }

    async fn m2m_explicit_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createManyPost(data: [
                        { id: 1, title: "p1" },
                        { id: 2, title: "p2" }
                    ])
                    { count }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation {
                    createManyCategory(data: [
                        { id: 1, name: "c1" },
                        { id: 2, name: "c2" },
                        { id: 3, name: "c3" }
                    ])
                    { count }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation {
                    createManyCategoriesOnPosts(data: [
                        { postId: 1, categoryId: 1 },
                        { postId: 1, categoryId: 2 },
                        { postId: 1, categoryId: 3 },
                        { postId: 2, categoryId: 2 },
                        { postId: 2, categoryId: 3 },
                    ])
                    { count }
                }"#,
            )
            .await?
            .assert_success();

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
            .query(format!("mutation {{ createOneUser(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();

        Ok(())
    }
}
