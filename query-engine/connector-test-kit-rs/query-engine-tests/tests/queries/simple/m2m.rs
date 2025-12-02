use query_engine_tests::*;

#[test_suite(schema(schema))]
mod m2m {
    use query_engine_tests::assert_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Post {
                #id(id, Int, @id)
                title   String
                content String @default("Wip")
                #m2m(categories, Category[], id, Int)
            }
    
            model Category {
                #id(id, Int, @id)
                name String

                #m2m(posts, Post[], id, Int)

                tags Tag[]
            }
            
            model Tag {
                #id(id, Int, @id)
                name String

                categoryId Int
                category   Category @relation(fields: [categoryId], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

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

    #[connector_test]
    async fn filtering_ordering(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findUniquePost(where: { id: 1 }) {
                    categories(
                        where: {
                            OR: [
                                { id: { in: [1] } },
                                { tags: { some: { name: "Cinema" } } }
                            ]
                        },
                        orderBy: { name: asc }
                    ) {
                        id
                        name
                    }
                }
            }"#),
          @r###"{"data":{"findUniquePost":{"categories":[{"id":2,"name":"Fiction"},{"id":1,"name":"Marketing"}]}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn basic_pagination(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findUniquePost(where: { id: 1 }) {
                    categories(
                        take: 1,
                        orderBy: { name: desc }
                    ) {
                        id
                        name
                    }
                }
            }"#),
          @r###"{"data":{"findUniquePost":{"categories":[{"id":1,"name":"Marketing"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                  findUniquePost(where: { id: 1 }) {
                      categories(
                          take: 1,
                          orderBy: { name: asc }
                      ) {
                          id
                          name
                      }
                  }
              }"#),
          @r###"{"data":{"findUniquePost":{"categories":[{"id":2,"name":"Fiction"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                  findUniquePost(where: { id: 1 }) {
                      categories(
                          skip: 1,
                          orderBy: { name: desc }
                      ) {
                          id
                          name
                      }
                  }
              }"#),
          @r###"{"data":{"findUniquePost":{"categories":[{"id":2,"name":"Fiction"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                  findUniquePost(where: { id: 1 }) {
                      categories(
                          skip: 1,
                          orderBy: { name: asc }
                      ) {
                          id
                          name
                      }
                  }
              }"#),
          @r###"{"data":{"findUniquePost":{"categories":[{"id":1,"name":"Marketing"}]}}}"###
        );

        Ok(())
    }

    fn m2m_sharing_same_row_schema() -> String {
        let schema = indoc! {
            r#"model User {
                #id(userId, BigInt, @id)
                #m2m(tags, Tag[], tagId, String)
              }
              
              model Tag {
                #id(tagId, String, @id, @default(uuid()))
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

    fn schema_16390() -> String {
        let schema = indoc! {
            r#"model Item {
                id         Int        @id @default(autoincrement())
                categories Category[]
                createdAt  DateTime   @default(now())
                updatedAt  DateTime?  @updatedAt
              }
              
              model Category {
                id        Int       @id @default(autoincrement())
                items     Item[]
                createdAt DateTime  @default(now())
                updatedAt DateTime? @updatedAt
              }"#
        };

        schema.to_owned()
    }

    // ! (https://github.com/prisma/prisma/issues/16390) - Skip on RLS::Query
    #[connector_test(schema(schema_16390), relation_mode = "prisma", only(Postgres))]
    async fn repro_16390(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneCategory(data: {}) { id } }"#);
        run_query!(
            &runner,
            r#"mutation { createOneItem(data: { categories: { connect: { id: 1 } } }) { id } }"#
        );
        run_query!(&runner, r#"mutation { deleteOneItem(where: { id: 1 }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findUniqueItem(relationLoadStrategy: join, where: { id: 1 })
                { id categories { id } }
            }"#),
          @r###"{"data":{"findUniqueItem":null}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findUniqueCategory(relationLoadStrategy: join, where: { id: 1 })
                { id items { id } }
            }"#),
          @r###"{"data":{"findUniqueCategory":{"id":1,"items":[]}}}"###
        );

        Ok(())
    }

    fn schema_28304() -> String {
        let schema = indoc! {
            r#"model AReallyLongModelName {
              id Int @id @default(autoincrement())

              verys AVeryVeryLongModelName[]
            }

            model AVeryVeryLongModelName {
              id Int @id @default(autoincrement())

              reallys AReallyLongModelName[]
            }"#
        };

        schema.to_owned()
    }

    // ! (https://github.com/prisma/prisma/issues/28304) - Many-to-many alias character limit
    #[connector_test(schema(schema_28304), only(Postgres))]
    async fn repro_28304(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneAReallyLongModelName(data: { verys: { create: {} } }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findFirstAReallyLongModelName(relationLoadStrategy: query) {
                    id
                    verys {
                        id
                    }
                }
            }"#),
          @r###"{"data":{"findFirstAReallyLongModelName":{"id":1,"verys":[{"id":1}]}}}"###
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
                            name: "Marketing",
                            tags: { create: { id: 1, name: "Business" } }
                        },
                        {
                            id: 2,
                            name: "Fiction",
                            tags: { create: { id: 2, name: "Cinema" } }
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
