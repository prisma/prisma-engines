use query_engine_tests::*;

#[test_suite(schema(schema), exclude(Sqlite("cfd1")))]
mod prisma_13097 {
    fn schema() -> String {
        r#"
        model TestModel {
                id      Int @id @map("_id")
                text    String
            }
        "#
        .to_owned()
    }

    #[connector_test]
    async fn filtering_with_dollar_values(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "mutation { createOneTestModel (data: {id: 1, text: \"foo\"}) { id, text }}"),
          @r###"{"data":{"createOneTestModel":{"id":1,"text":"foo"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, "mutation { createOneTestModel (data: {id: 2, text: \"$foo\"}) { id, text }}"),
          @r###"{"data":{"createOneTestModel":{"id":2,"text":"$foo"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, "query { findFirstTestModel (where: {text: \"foo\"}) { id, text }}"),
          @r###"{"data":{"findFirstTestModel":{"id":1,"text":"foo"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, "query { findFirstTestModel (where: {text: \"$foo\"}) { id, text }}"),
          @r###"{"data":{"findFirstTestModel":{"id":2,"text":"$foo"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, "query { findManyTestModel { id, text }}"),
          @r###"{"data":{"findManyTestModel":[{"id":1,"text":"foo"},{"id":2,"text":"$foo"}]}}"###
        );

        Ok(())
    }
}
