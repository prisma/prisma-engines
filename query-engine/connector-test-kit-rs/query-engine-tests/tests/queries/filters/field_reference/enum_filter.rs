use query_engine_tests::*;

#[test_suite(schema(schema))]
mod enum_filter {
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)

              enum TestEnum?
              enum2 TestEnum[]
            }
            
            enum TestEnum {
              a
              b
              c
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(capabilities(Enums, ScalarLists))]
    async fn inclusion_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { enum: { in: { _ref: "enum2", _container: "TestModel" } } }) { id enum enum2 }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"enum":"a","enum2":["a","b"]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { enum: { notIn: { _ref: "enum2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { enum: { not: { in: { _ref: "enum2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    pub async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation { createOneTestModel(data: {
                  id: 1,
                  enum: a,
                  enum2: [a, b]
              }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
            mutation { createOneTestModel(data: {
                id: 2,
                enum: b,
                enum2: [a, c]
            }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
            .await?
            .assert_success();

        Ok(())
    }
}
