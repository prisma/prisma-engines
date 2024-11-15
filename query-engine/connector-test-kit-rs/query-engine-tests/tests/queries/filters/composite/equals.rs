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

        // Implicit equal
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: [ { a_1: "Test", a_2: 0 } ]
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":6},{"id":7}]}}"###
        );

        // Implicit
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                      findManyTestModel(where: {
                          NOT: [
                              {
                                  to_many_as: [ { a_1: "Test", a_2: 0 } ]
                              }
                          ]
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":6},{"id":7}]}}"###
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

        // Implicit
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: []
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        // Implicit
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        NOT: [
                            {
                              to_many_as: []
                            }
                        ]
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // No object coercion
    // TODO: This test is ignored because the JSON protocol required to enable the object syntax for equality on composite lists.
    // TODO: It should be enabled again once we remove the object shorthand syntaxes.
    #[connector_test]
    async fn single_object(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"{
                findManyTestModel(where: { to_many_as: { equals: { a_1: "Test", a_2: 0 } }}) {
                    id
                }
            }"#,
            2009,
            "Invalid argument type"
        );

        Ok(())
    }
}

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod to_one {
    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            a_1: "foo1"
                            a_2: 1
                            b: { b_field: "b_nested_1", c: { c_field: "c_field default" } }
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                      findManyTestModel(where: {
                          NOT: [{ a: {
                              a_1: "foo1"
                              a_2: 1
                              b: { b_field: "b_nested_1", c: { c_field: "c_field default" } }
                          } }]
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn field_order_matters(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            a_2: 1
                            a_1: "foo1"
                            b: { b_field: "b_nested_1", c: { c_field: "c_field default" } }
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }
}
