use query_engine_tests::*;

#[test_suite(schema(schemas::posts_categories))]
mod m2m {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn fetch_only_associated(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

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

    fn m2m_sharing_same_row_schema() -> String {
        let schema = indoc! {
            r#"model User {
                userId BigInt @id
                #m2m(tags, Tag[], tagId, String)
              }
              
              model Tag {
                tagId String @id @default(uuid())
                name  String
                #m2m(users, User[], userId, BigInt)
              }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(m2m_sharing_same_row_schema))]
    async fn m2m_sharing_same_row(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(
                  data: {
                    userId: "1"
                    tags: {
                      create: [{ tagId: "1", name: "tag_a" }, { tagId: "2", name: "tag_b" }]
                    }
                  }
                ) {
                  userId
                }
              }              
          "#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTag  { tagId users { userId } } }"#),
          @r###"{"data":{"findManyTag":[{"tagId":"1","users":[{"userId":"1"}]},{"tagId":"2","users":[{"userId":"1"}]}]}}"###
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
