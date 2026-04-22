use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(InsensitiveFilters, JsonFiltering))]
mod insensitive_json {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, String, @id, @default(cuid()))
              json Json
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn string_matcher(runner: Runner) -> TestResult<()> {
        create_row(&runner, "a test").await?;
        create_row(&runner, "A Test").await?;
        create_row(&runner, "b test").await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { path: ["nested"], equals: "\"a test\"", mode: insensitive } }) { json }}"#),
          @r###"{"data":{"findManyTestModel":[{"json":"{\"nested\":\"a test\"}"},{"json":"{\"nested\":\"A Test\"}"}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, s: &str) -> TestResult<()> {
        runner
            .query(format!(
                r#"mutation {{ createOneTestModel(data: {{ json: "{{ \"nested\": \"{s}\" }}" }}) {{ id }} }}"#,
            ))
            .await?
            .assert_success();

        Ok(())
    }
}
