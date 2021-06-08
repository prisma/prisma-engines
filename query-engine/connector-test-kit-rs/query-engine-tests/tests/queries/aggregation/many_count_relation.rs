use query_engine_tests::*;

#[test_suite(schema(schema))]
mod many_count_rel {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Post {
              #id(id, Int, @id)
              title      String
              comments   Comment[]
              #m2m(categories, Category[], Int)
            }
            
            model Comment {
              #id(id, Int, @id)
              post    Post    @relation(fields: [postId], references: [id])
              postId  Int
            }
            
            model Category {
              #id(id, Int, @id)
              #m2m(posts, Post[], Int)
            }"#
        };

        schema.to_owned()
    }

    // "Counting with no records in the database" should "return 0"
    #[connector_test(exclude(MongoDb))] // TODO(dom): Not working on mongo
    async fn no_rel_records(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, title: "a" }"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyPost {
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"_count":{"comments":0,"categories":0}}]}}"###
        );

        Ok(())
    }

    //"Counting one2m and m2m records" should "work"
    #[connector_test(exclude(MongoDb))] // TODO(dom): Not working on mongo
    async fn count_one2m_m2m(runner: &Runner) -> TestResult<()> {
        // 1 comment / 2 categories
        create_row(
            runner,
            r#"{
          id: 1,
          title: "a",
          comments: { create: [{id: 1}] },
          categories: { create: [{id: 1}, {id: 2}] },
            }"#,
        )
        .await?;
        // 3 comment / 4 categories
        create_row(
            runner,
            r#"{
          id: 2,
          title: "b",
          comments: { create: [{id: 2}, {id: 3}, {id: 4}] },
          categories: { create: [{id: 3}, {id: 4}, {id: 5}, {id: 6}] }
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: { id: asc }) {
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"_count":{"comments":1,"categories":2}},{"_count":{"comments":3,"categories":4}}]}}"###
        );

        Ok(())
    }

    // "Counting with some records and filters" should "not affect the count"
    #[connector_test(exclude(MongoDb))] // TODO(dom): Not working on mongo
    async fn count_with_filters(runner: &Runner) -> TestResult<()> {
        // 4 comment / 4 categories
        create_row(
            runner,
            r#"{
                  id: 1,
                  title: "a",
                  comments: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
                  categories: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] }
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(where: { id: 1 }) {
              comments(cursor: { id: 1 }, take: 1) { id }
              categories(cursor: { id: 1 }, take: 1) { id }
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"comments":[{"id":1}],"categories":[{"id":1}],"_count":{"comments":4,"categories":4}}]}}"###
        );

        Ok(())
    }

    fn schema_nested() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name  String
              posts Post[]
            }
            
            model Post {
              #id(id, Int, @id)
              title    String
              user     User      @relation(fields: [userId], references: [id])
              userId   Int
              #m2m(comments, Comment[], Int)
              #m2m(tags, Tag[], Int)
            }
            
            model Comment {
              #id(id, Int, @id)
              body   String
              post   Post   @relation(fields: [postId], references: [id])
              postId Int
              #m2m(tags, Tag[], Int)
            }
            
            model Tag {
              #id(id, Int, @id)
              name     String
              #m2m(posts, Post[], Int)
              #m2m(comments, Comment[], Int)
            }"#
        };

        schema.to_owned()
    }

    // Counting nested one2m and m2m should work
    // TODO(dom): Not working on mongo
    #[connector_test(schema(schema_nested), exclude(MongoDb))]
    async fn nested_count_one2m_m2m(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
          createOneUser(
            data: {
              id: 1,
              name: "Bob"
              posts: {
                create: {
                  id: 1,
                  title: "Wooow!"
                  comments: {
                    create: {
                      id: 1,
                      body: "Amazing",
                      tags: { create: [{ id: 1, name: "LALA" }, { id: 2, name: "LOLO" }] } }
                  },
                  tags: {
                    create: [{ id: 3, name: "A"}, {id: 4, name: "B"}, {id: 5, name: "C"}]
                  }
                }
              }
            }
          ) {
            id
          }
        }
        "#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyUser {
            name
            posts {
              title
              comments {
                body
                tags {
                  name
                }
                _count {
                  tags
                }
              }
              tags {
                name
              }
              _count {
                comments
                tags
              }
            }
            _count {
              posts
            }
          } }"#),
          @r###"{"data":{"findManyUser":[{"name":"Bob","posts":[{"title":"Wooow!","comments":[{"body":"Amazing","tags":[{"name":"LALA"},{"name":"LOLO"}],"_count":{"tags":2}}],"tags":[{"name":"A"},{"name":"B"},{"name":"C"}],"_count":{"comments":1,"tags":3}}],"_count":{"posts":1}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOnePost(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
