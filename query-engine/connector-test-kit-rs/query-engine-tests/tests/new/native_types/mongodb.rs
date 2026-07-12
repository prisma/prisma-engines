use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod mongodb {
    use query_engine_tests::{Runner, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, String, @id)
            }"#
        };

        schema.to_owned()
    }

    // Regression test for native type coercions not applying correctly to lists.
    #[connector_test]
    async fn native_type_list_coercion(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneTestModel(data: { id: "609675d400e7693e0090e48c" }) { id }}"#)
            .await?
            .assert_success();

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyTestModel(where: { id: { in: ["609675d400e7693e0090e48c", "507f1f77bcf86cd799439011"] } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":"609675d400e7693e0090e48c"}]}}"###
        );

        Ok(())
    }
}
