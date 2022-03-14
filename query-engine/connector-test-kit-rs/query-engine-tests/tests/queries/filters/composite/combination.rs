use super::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod combination {
    // Over to-one to to-many (both composites)
    #[connector_test]
    async fn com_to_one_2_to_many(runner: Runner) -> TestResult<()> {
        create_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_b: {
                      is: {
                          b_to_many_cs: {
                              every: {
                                  c_field: { gt: 0 }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":6},{"id":8},{"id":9},{"id":10},{"id":11}]}}"###
        );

        Ok(())
    }
}
