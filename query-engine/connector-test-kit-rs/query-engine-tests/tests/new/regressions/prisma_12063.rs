use query_engine_tests::*;

#[test_suite(schema(generic))]
mod prisma_12063 {
    #[connector_test]
    async fn and_or_undefined(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneTestModel(data: { id: 33 }) { id } }"#)
            .await?
            .assert_success();

        let query_a = r#"
            query {
              findManyTestModel(where: {
                OR: [
                  {

                  }
                ]
              }) {
                id
              }
            }
        "#;

        let query_b = r#"
            query {
              findManyTestModel(where: {
                AND: {
                  OR: [
                    {

                    }
                  ]
                }
              }) {
                id
              }
            }
        "#;

        let expected_result = "{\"data\":{\"findManyTestModel\":[{\"id\":33}]}}";
        let query_a_result = run_query!(runner, query_a);
        let query_b_result = run_query!(runner, query_b);

        assert_eq!(query_a_result, expected_result);
        assert_eq!(query_b_result, expected_result);

        Ok(())
    }
}
