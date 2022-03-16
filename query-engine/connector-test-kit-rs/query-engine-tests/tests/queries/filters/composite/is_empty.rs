use super::*;

#[test_suite(schema(to_many_composites), only(MongoDB))]
mod is_empty {
    use query_engine_tests::run_query;

    #[connector_test]
    async fn basic_empty_check(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        isEmpty: true
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          isEmpty: false
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn negation(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    NOT: [
                        {
                            to_many_as: {
                                isEmpty: true
                            }
                        }
                    ]
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      NOT: [
                          {
                            to_many_as: {
                                isEmpty: false
                            }
                          }
                      ]
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn silly_combinations(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    AND: [
                        {
                            to_many_as: {
                                isEmpty: true
                            }
                        },
                        {
                            to_many_as: {
                                isEmpty: false
                            }
                        }
                    ]

                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      OR: [
                          {
                              to_many_as: {
                                  isEmpty: true
                              }
                          },
                          {
                              to_many_as: {
                                  isEmpty: false
                              }
                          }
                      ]

                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }
}
