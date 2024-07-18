use query_engine_tests::*;

#[test_suite(schema(schema))]
mod uniq_count_rel {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Post {
              #id(id, Int, @id)
              title      String
              comments   Comment[]
              #m2m(categories, Category[], id, Int)
            }

            model Comment {
              #id(id, Int, @id)
              post    Post    @relation(fields: [postId], references: [id])
              postId  Int
            }

            model Category {
              #id(id, Int, @id)
              #m2m(posts, Post[], id, Int)
            }"#
        };

        schema.to_owned()
    }

    // "Counting with no records in the database" should "return 0"
    #[connector_test]
    async fn no_rel_records(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "a" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findUniquePost(where: { id: 1 }) {
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"_count":{"comments":0,"categories":0}}}}"###
        );

        Ok(())
    }

    //"Counting one2m and m2m records" should "work"
    #[connector_test]
    async fn count_one2m_m2m(runner: Runner) -> TestResult<()> {
        // 1 comment / 2 categories
        create_row(
            &runner,
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
            &runner,
            r#"{
          id: 2,
          title: "b",
          comments: { create: [{id: 2}, {id: 3}, {id: 4}] },
          categories: { create: [{id: 3}, {id: 4}, {id: 5}, {id: 6}] },
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findUniquePost(where: { id: 1 }) {
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"_count":{"comments":1,"categories":2}}}}"###
        );

        Ok(())
    }

    // Counting with cursor should not affect the count
    #[connector_test]
    async fn count_with_cursor(runner: Runner) -> TestResult<()> {
        // 4 comment / 4 categories
        create_row(
            &runner,
            r#"{
                  id: 1,
                  title: "a",
                  comments: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
                  categories: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniquePost(where: { id: 1 }) {
              comments(cursor: { id: 1 }, take: 1) { id }
              categories(cursor: { id: 1 }, take: 1) { id }
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"comments":[{"id":1}],"categories":[{"id":1}],"_count":{"comments":4,"categories":4}}}}"###
        );

        Ok(())
    }

    // Counting with take should not affect the count
    #[connector_test]
    async fn count_with_take(runner: Runner) -> TestResult<()> {
        // 4 comment / 4 categories
        create_row(
            &runner,
            r#"{
                  id: 1,
                  title: "a",
                  comments: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
                  categories: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniquePost(where: { id: 1 }) {
              comments(take: 1) { id }
              categories(take: 1) { id }
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"comments":[{"id":1}],"categories":[{"id":1}],"_count":{"comments":4,"categories":4}}}}"###
        );

        Ok(())
    }

    // Counting with skip should not affect the count
    #[connector_test]
    async fn count_with_skip(runner: Runner) -> TestResult<()> {
        // 4 comment / 4 categories
        create_row(
            &runner,
            r#"{
                  id: 1,
                  title: "a",
                  comments: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
                  categories: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniquePost(where: { id: 1 }) {
              comments(skip: 2) { id }
              categories(skip: 2) { id }
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"comments":[{"id":3},{"id":4}],"categories":[{"id":3},{"id":4}],"_count":{"comments":4,"categories":4}}}}"###
        );

        Ok(())
    }

    // Counting with filters should not affect the count
    #[connector_test]
    async fn count_with_filters(runner: Runner) -> TestResult<()> {
        // 4 comment / 4 categories
        create_row(
            &runner,
            r#"{
                  id: 1,
                  title: "a",
                  comments: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
                  categories: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniquePost(where: { id: 1 }) {
              comments(where: { id: 2}) { id }
              categories(where: { id: 2}) { id }
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"comments":[{"id":2}],"categories":[{"id":2}],"_count":{"comments":4,"categories":4}}}}"###
        );

        Ok(())
    }

    // Counting with distinct should not affect the count
    #[connector_test]
    async fn count_with_distinct(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                  id: 1,
                  title: "a",
                  categories: { create: { id: 1 } }
            }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{
                  id: 2,
                  title: "a",
                  categories: { connect: { id: 1 } }
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{
              findUniqueCategory(where: { id: 1 }) {
                posts(distinct: title) { id }
                _count { posts }
              }
            }"#),
            @r###"{"data":{"findUniqueCategory":{"posts":[{"id":1}],"_count":{"posts":2}}}}"###
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
            comments Comment[]
            #m2m(tags, Tag[], id, Int)
          }

          model Comment {
            #id(id, Int, @id)
            body   String
            post   Post   @relation(fields: [postId], references: [id])
            postId Int
            #m2m(tags, Tag[], id, Int)
          }

          model Tag {
            #id(id, Int, @id)
            name     String
            #m2m(posts, Post[], id, Int)
            #m2m(comments, Comment[], id, Int)
          }"#
        };

        schema.to_owned()
    }

    // Counting nested one2m and m2m should work
    #[connector_test(schema(schema_nested))]
    async fn nested_count_one2m_m2m(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
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
          run_query!(&runner, r#"{ findUniqueUser(where: { id: 1 }) {
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
          @r###"{"data":{"findUniqueUser":{"name":"Bob","posts":[{"title":"Wooow!","comments":[{"body":"Amazing","tags":[{"name":"LALA"},{"name":"LOLO"}],"_count":{"tags":2}}],"tags":[{"name":"A"},{"name":"B"},{"name":"C"}],"_count":{"comments":1,"tags":3}}],"_count":{"posts":1}}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOnePost(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
