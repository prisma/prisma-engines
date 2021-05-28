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
    // TODO(dom): Not working on mongo
    // TODO(dom): {"errors":[{"error":"called `Option::unwrap()` on a `None` value","user_facing_error":{"is_panic":true,"message":"called `Option::unwrap()` on a `None` value","backtrace":null}}]}
    #[connector_test(exclude(MongoDb))]
    async fn no_rel_records(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, title: "a" }"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findUniquePost(where: { id: 1 }) {
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"_count":{"comments":0,"categories":0}}}}"###
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
          categories: { create: [{id: 3}, {id: 4}, {id: 5}, {id: 6}] },
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findUniquePost(where: { id: 1 }) {
              _count { comments categories }
            }
          }"#),
          @r###"{"data":{"findUniquePost":{"_count":{"comments":1,"categories":2}}}}"###
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
                  categories: { create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}] },
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
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

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOnePost(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
