use query_engine_tests::*;

#[test_suite(schema(schema))]
mod rel_filter_ordering {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Blog {
              #id(id, Int, @id)
              title String
              score Int

              #m2m(labels, Label[], Int)
            }

            model Label {
              #id(id, Int, @id)
              text String @unique

              #m2m(blogs, Blog[], Int)
            }"#
        };

        schema.to_owned()
    }

    // "Using relational filters" should "return items in the specified order"
    #[connector_test]
    async fn rel_filters(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog(take: 2, orderBy: { score: desc }) { title, score }}"#),
          @r###"{"data":{"findManyBlog":[{"title":"blog_1","score":30},{"title":"blog_1","score":20}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyBlog (take: 2, orderBy: { score: desc }, where:{ labels: { some: { text: { equals: "x" }}}}) { title, score }}"#),
          @r###"{"data":{"findManyBlog":[{"title":"blog_1","score":30},{"title":"blog_1","score":20}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation {createOneLabel(data: {id: 1, text: "x"}) { text }}"#)
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation {createOneBlog(data: {id: 1, title: "blog_1", score: 10,labels: {connect: {text: "x"}}}) {title}}"#,
            )
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneBlog(data: {id: 2, title: "blog_1", score: 20,labels: {connect: {text: "x"}}}) {title}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneBlog(data: {id: 3, title: "blog_1", score: 30,labels: {connect: {text: "x"}}}) {title}}"#)
            .await?
            .assert_success();

        Ok(())
    }
}
