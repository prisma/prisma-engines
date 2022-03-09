use super::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod to_many {
    #[connector_test]
    async fn basic_equality(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          equals: [ { a_1: "Test", a_2: 0 } ]
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        NOT: [
                            {
                                to_many_as: {
                                    equals: [ { a_1: "Test", a_2: 0 } ]
                                }
                            }
                        ]
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn field_order_matters(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // Establish baseline
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: {
                            equals: [ { a_1: "Test", a_2: 0 } ]
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        // Actual test
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          equals: [ { a_2: 0, a_1: "Test" } ]
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn object_order_matters(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // Establish baseline
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          equals: [
                              { a_1: "test", a_2: -5 },
                              { a_1: "Test", a_2: 0 }
                          ]
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":4}]}}"###
        );

        // Actual test
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: {
                            equals: [
                                { a_1: "Test", a_2: 0 },
                                { a_1: "test", a_2: -5 }
                            ]
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_comparison(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        equals: []
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
                      NOT: [
                          {
                            to_many_as: {
                                equals: []
                            }
                          }
                      ]
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    // No object coercion
    #[connector_test]
    async fn single_object(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        equals: { a_1: "Test", a_2: 0 }
                    }
                }) {
                    id
                }
            }"#,
            2009,
            "Query parsing/validation error at `Query.findManyTestModel.where.TestModelWhereInput.to_many_as.CompositeACompositeListFilter.equals`: Value types mismatch. Have: Object({\"a_1\": String(\"Test\"), \"a_2\": Int(0)}), want: Object(CompositeAObjectEqualityInput)"
        );

        Ok(())
    }
}
