use query_engine_tests::*;

#[test_suite(schema(schema))]
mod many_relation {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Blog {
                #id(id, String, @id, @default(cuid()))
                name  String
                posts Post[]
            }

            model Post {
                #id(id, String, @id, @default(cuid()))
                title      String
                popularity Int
                blog_id    String
                blog       Blog      @relation(fields: [blog_id], references: [id])
                comments   Comment[]
            }

            model Comment {
                #id(id, String, @id, @default(cuid()))
                text    String
                likes   Int
                post_id String
                post    Post   @relation(fields: [post_id], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn simple_scalar_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog { posts(where: { popularity: { gte: 5 }}, orderBy: { id: asc }) { title }}}"#),
          @r###"{"data":{"findManyBlog":[{"posts":[{"title":"post 1"}]},{"posts":[{"title":"post 3"}]}]}}"###
        );

        Ok(())
    }

    // 1 level to-one-relation filter
    #[connector_test]
    async fn l1_1_rel(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyPost(where: { blog: { is: { name: { equals: "blog 1" }}}}, orderBy: { id: asc }) { title }}"#),
          @r###"{"data":{"findManyPost":[{"title":"post 1"},{"title":"post 2"}]}}"###
        );

        Ok(())
    }

    // 1 level to-many-relation filter, `some` operation.
    #[connector_test]
    async fn l1_m_rel_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { popularity: { gte: 5 }}}}, orderBy: { id: asc }) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"},{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { popularity: { gte: 50 }}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { AND: [{ title: { equals: "post 1" }}, { title: { equals: "post 2" }}]}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, indoc!{ r#"
              query {
                findManyBlog(
                  where: {
                    AND: [
                      { posts: { some: { title: { equals: "post 1" } } } }
                      { posts: { some: { title: { equals: "post 2" } } } }
                    ]
                  }
                ) {
                  name
                }
              }
            "# }),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, indoc!{ r#"
              query {
                findManyBlog(
                  where: {
                    posts: {
                      some: {
                        AND: [{ title: { equals: "post 1" } }, { popularity: { gte: 2 } }]
                      }
                    }
                  }
                ) {
                  name
                }
              }
            "# }),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        Ok(())
    }

    // 1 level to-many-relation filter, `every` operation.
    #[connector_test]
    async fn l1_m_rel_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { popularity: { gte: 2 }}}}, orderBy: { id: asc }) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"},{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { popularity: { gte: 3 }}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { AND: [{ title: { equals: "post 1" }}, { title: { equals: "post 2" }}]}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        Ok(())
    }

    // 1 level to-many-relation filter, `none` operation.
    #[connector_test]
    async fn l1_m_rel_none(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { popularity: { gte: 50 }}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { popularity: { gte: 5 }}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        Ok(())
    }

    // 2 levels to-many-relation filter, `some`/`some` combination.
    #[connector_test]
    async fn l2_m_rel_some_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { comments: { some: { likes: { equals: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { comments: { some: { likes: { equals: 1 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        Ok(())
    }

    // 2 levels to-many-relation filter, all combinations.
    #[connector_test]
    async fn l2_m_rel_all(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // some|every
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { comments: { every: { likes: { gte: 0 }}}}}}, orderBy: { id: asc }) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"},{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {findManyBlog(where: { posts: { some: { comments: { every: { likes: { equals: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        // some|none
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { comments: { none: { likes: { equals: 0 }}}}}}, orderBy: { id: asc }) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"},{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { some: { comments: { none: { likes: { gte: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        // every|some
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { comments: { some: { likes: { equals: 10 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { comments: { some: { likes: { equals: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        // every|every
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { comments: { every: { likes: { gte: 0 }}}}}}, orderBy: { id: asc }) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"},{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { comments: { every: { likes: { equals: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        // every|none
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { comments: { none: { likes: { gte: 100 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { every: { comments: { none: { likes: { equals: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 2"}]}}"###
        );

        // none|some
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { comments: { some: { likes: { gte: 100 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { comments: { some: { likes: { equals: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 2"}]}}"###
        );

        // none|every
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { comments: { every: { likes: { gte: 11 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { comments: { every: { likes: { gte: 0 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[]}}"###
        );

        // none|none
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { comments: { none: { likes: { gte: 0 }}}}}}, orderBy: { id: asc }) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 1"},{"name":"blog 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(where: { posts: { none: { comments: { none: { likes: { gte: 11 }}}}}}) { name }}"#),
          @r###"{"data":{"findManyBlog":[{"name":"blog 2"}]}}"###
        );

        Ok(())
    }

    // Note: Only the original author knows why this is considered crazy.
    #[connector_test]
    async fn crazy_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyPost(
                where: {
                  blog: {
                    is: {
                      posts: { some: { popularity: { gte: 5 } } }
                      name: { contains: "Blog 1" }
                    }
                  }
                  AND: [
                    { comments: { none: { likes: { gte: 5 } } } },
                    { comments: { some: { likes: { lte: 2 } } } }
                  ]
                }
              ) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManyPost":[]}}"###
        );

        Ok(())
    }

    fn special_case_schema() -> String {
        let schema = indoc! {
            r#"
            model Post {
                #id(id, String, @id, @default(cuid()))
                #m2m(authors, AUser[], String)
                title   String  @unique
            }

            model AUser {
                #id(id, String, @id, @default(cuid()))
                #m2m(posts, Post[], String)
                name  String @unique
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(special_case_schema))]
    async fn m2m_join_relation_1level(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOnePost(data: { title: "Title1" }) { title }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOnePost(data: { title: "Title2" }) { title }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneAUser(data: { name: "Author1" }) { name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneAUser(data: { name:"Author2" }) { name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { updateOneAUser(where: { name: "Author1" }, data: { posts: { connect: [{ title: "Title1" }, { title: "Title2" }]}}) { name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { updateOneAUser(where: { name: "Author2" }, data: { posts: { connect:[{ title: "Title1" }, { title: "Title2" }]}}) { name }}"#)
            .await?
            .assert_success();

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyAUser (orderBy: { id: asc }){ name, posts(orderBy: { id: asc }) { title }}}"#),
            @r###"{"data":{"findManyAUser":[{"name":"Author1","posts":[{"title":"Title1"},{"title":"Title2"}]},{"name":"Author2","posts":[{"title":"Title1"},{"title":"Title2"}]}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyPost(orderBy: { id: asc }) { title, authors (orderBy: { id: asc }){ name }}}"#),
            @r###"{"data":{"findManyPost":[{"title":"Title1","authors":[{"name":"Author1"},{"name":"Author2"}]},{"title":"Title2","authors":[{"name":"Author1"},{"name":"Author2"}]}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyAUser(where: { name: { startsWith: "Author2" }, posts: { some: { title: { endsWith: "1" }}}}, orderBy: { id: asc }) { name, posts(orderBy: { id: asc }) { title }}}"#),
            @r###"{"data":{"findManyAUser":[{"name":"Author2","posts":[{"title":"Title1"},{"title":"Title2"}]}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneBlog(
                  data: {
                    name: "blog 1"
                    posts: {
                      create: [
                        {
                          title: "post 1"
                          popularity: 10
                          comments: {
                            create: [
                              { text: "comment 1", likes: 0 }
                              { text: "comment 2", likes: 5 }
                              { text: "comment 3", likes: 10 }
                            ]
                          }
                        }
                        {
                          title: "post 2"
                          popularity: 2
                          comments: { create: [{ text: "comment 4", likes: 10 }] }
                        }
                      ]
                    }
                  }
                ) {
                  name
                }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneBlog(
                  data: {
                    name: "blog 2"
                    posts: {
                      create: [
                        {
                          title: "post 3"
                          popularity: 1000
                          comments: { create: [{ text: "comment 5", likes: 1000 }] }
                        }
                      ]
                    }
                  }
                ) {
                  name
                }
              }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}
