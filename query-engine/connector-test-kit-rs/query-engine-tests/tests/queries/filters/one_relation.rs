use query_engine_tests::*;

#[test_suite(schema(schema))]
mod one_relation {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Blog {
                #id(id, String, @id, @default(cuid()))
                name String
                post Post?
            }

            model Post {
                #id(id, String, @id, @default(cuid()))
                title      String
                popularity Int
                blogId     String? @unique
                blog       Blog?    @relation(fields: [blogId], references: [id])
                comment    Comment?
            }

            model Comment {
                #id(id, String, @id, @default(cuid()))
                text   String
                likes  Int
                postId String? @unique
                post   Post?   @relation(fields: [postId], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_scalar(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyPost(where: { title: { equals: "post 2" }}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 2"}]}}"###
        );

        Ok(())
    }

    // 1 level to-one-relation filters.
    #[connector_test]
    async fn l1_one_rel(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyPost(where: { title: { equals: "post 2" }}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyPost(where:{blog:{is:{name:{equals: "blog 1"}}}}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyBlog(where: { post: { is:{popularity: { gte: 100 }}}}){name}}"#),
            @r###"{"data":{"findManyBlog":[{"name":"blog 2"},{"name":"blog 3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyBlog(where: { post: { is:{popularity: { gte: 500 }}}}){name}}"#),
            @r###"{"data":{"findManyBlog":[{"name":"blog 3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyBlog(where: { post: { isNot:{popularity: { gte: 500 }}}}){name}}"#),
            @r###"{"data":{"findManyBlog":[{"name":"blog 1"},{"name":"blog 2"}]}}"###
        );

        runner
            .query(r#"mutation { createOnePost(data: { title: "Post 4" popularity: 5 }) { title } }"#)
            .await?
            .assert_success();

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyPost(where:{blog: { is: null}}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"Post 4"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyPost(where:{blog: { isNot: null}}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 1"},{"title":"post 2"},{"title":"post 3"}]}}"###
        );

        Ok(())
    }

    // 1 level to-one-relation filters with shorthands.
    #[connector_test]
    async fn l1_one_rel_shorthands(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyPost(where: { title: { equals: "post 2" }}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyPost(where: { blog: { name: { equals: "blog 1" }}}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyBlog(where: { post: { popularity: { gte: 100 }}}){name}}"#),
            @r###"{"data":{"findManyBlog":[{"name":"blog 2"},{"name":"blog 3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyBlog(where: { post: { popularity: { gte: 500 }}}){name}}"#),
            @r###"{"data":{"findManyBlog":[{"name":"blog 3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyPost(where: { blog: { name: { equals: "blog 1" }}}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 1"}]}}"###
        );

        runner
            .query(r#"mutation { createOnePost(data: { title: "Post 4" popularity: 5 }) { title } }"#)
            .await?
            .assert_success();

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyPost(where: { blog: null }) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"Post 4"}]}}"###
        );

        Ok(())
    }

    // 2 levels to-one-relation filter.
    #[connector_test]
    async fn l2_one_rel(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyPost(where: { title: { equals: "post 2" }}) { title }}"#),
            @r###"{"data":{"findManyPost":[{"title":"post 2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyBlog(where: { post: { is:{comment: {is:{likes: {equals:10}}}}}}){name}}"#),
            @r###"{"data":{"findManyBlog":[{"name":"blog 1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyBlog(where: { post: { is:{comment:{is:{likes:{equals:1000}}}}}}){name}}"#),
            @r###"{"data":{"findManyBlog":[{"name":"blog 3"}]}}"###
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
                      post: { is: { popularity: { gte: 10 } } }
                      name: { contains: "blog 1" }
                    }
                  }
                  comment: { is: { likes: { gte: 5 }, likes: { lte: 200 } } }
                }
              ) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManyPost":[{"title":"post 1"}]}}"###
        );

        Ok(())
    }

    fn special_case_schema() -> String {
        let schema = indoc! {
            r#"
            model Post {
                #id(id, String, @id, @default(cuid()))
                title   String @unique
                author  AUser?
            }

            model AUser {
                #id(id, String, @id, @default(cuid()))
                name   String  @unique
                int    Int?
                postId String? @unique
                post   Post?   @relation(fields: [postId], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(special_case_schema), exclude(SqlServer))]
    async fn one2one_join_relation_1level(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOnePost(data: { title:"Title1"}) { title }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOnePost(data: { title:"Title2"}) { title }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneAUser(data: {name: "Author1", int: 5 }) { name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneAUser(data: {name: "Author2", int: 4 }) { name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { updateOneAUser(where: { name: "Author1" }, data: { post: { connect: { title: "Title1" }}}) { name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { updateOneAUser(where: { name: "Author2" }, data: { post: { connect: { title: "Title2" }}}) { name }}"#)
            .await?
            .assert_success();

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyAUser { name, post { title }}}"#),
            @r###"{"data":{"findManyAUser":[{"name":"Author1","post":{"title":"Title1"}},{"name":"Author2","post":{"title":"Title2"}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyPost { title, author { name }}}"#),
            @r###"{"data":{"findManyPost":[{"title":"Title1","author":{"name":"Author1"}},{"title":"Title2","author":{"name":"Author2"}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyAUser(where: { post: { is: { title: { endsWith: "1" }}}, name: { startsWith: "Author" }, int: { equals: 5}}) { name, post { title }}}"#),
            @r###"{"data":{"findManyAUser":[{"name":"Author1","post":{"title":"Title1"}}]}}"###
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
                            post: {
                                create: {
                                    title: "post 1"
                                    popularity: 10
                                    comment: { create: { text: "comment 1", likes: 10 } }
                                }
                            }
                        }
                    ) { name }
                }
            "#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation {
                    createOneBlog(
                        data: {
                            name: "blog 2"
                            post: {
                                create: {
                                    title: "post 2"
                                    popularity: 100
                                    comment: { create: { text: "comment 2", likes: 100 } }
                                }
                            }
                        }
                    ) { name }
                }
            "#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation {
                    createOneBlog(
                        data: {
                            name: "blog 3"
                            post: {
                                create: {
                                    title: "post 3"
                                    popularity: 1000
                                    comment: { create: { text: "comment 3", likes: 1000 } }
                                }
                            }
                        }
                    ) { name }
                }
            "#
            })
            .await?
            .assert_success();

        Ok(())
    }
}
