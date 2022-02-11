use query_engine_tests::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod create {
    use query_engine_tests::run_query;

    /// Using explicit `set` operators, create (deeply nested) composite lists.
    #[connector_test]
    async fn set_create(runner: Runner) -> TestResult<()> {
        // Single-object shorthand for lists.
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                to_many_as: { set: { a_1: "a1", a_2: null } }
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
            }
          }
          "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":null}]}}}"###
        );

        // Full: set + list wrapper
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 2
                to_many_as: { set: [{ a_1: "a1", a_2: null }] }
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
            }
          }
        "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":null}]}}}"###
        );

        // Many items at once
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                  createOneTestModel(
                    data: {
                      id: 3
                      to_many_as: {
                        set: [
                          {
                            a_1: "1"
                            a_2: 1
                          },
                          {
                            a_1: "2"
                            a_2: 2
                          },
                          {
                            a_1: "3"
                            a_2: 3
                          }
                        ]
                      }
                    }
                  ) {
                    to_many_as {
                      a_1
                      a_2
                    }
                  }
                }
              "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"1","a_2":1},{"a_1":"2","a_2":2},{"a_1":"3","a_2":3}]}}}"###
        );

        Ok(())
    }

    /// Using shorthand operators, create (deeply nested) composite lists.
    #[connector_test]
    async fn shorthand_set_create(runner: Runner) -> TestResult<()> {
        // Single-object shorthand for lists.
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(
                data: {
                  id: 1
                  to_many_as: { a_1: "a1", a_2: null }
                  to_one_b: { b_to_many_cs: { c_field: 15 } }
                }
              ) {
                to_many_as {
                  a_1
                  a_2
                }
                to_one_b {
                  b_field
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
            "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":null}],"to_one_b":{"b_field":10,"b_to_many_cs":[{"c_field":15}]}}}}"###
        );

        // Shorthand with explicit list wrapper.
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(
                data: {
                  id: 2
                  to_many_as: [{ a_1: "a1", a_2: null }]
                  to_one_b: { b_to_many_cs: [{ c_field: 15 }] }
                }
              ) {
                to_many_as {
                  a_1
                  a_2
                }
                to_one_b {
                  b_field
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
            "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":null}],"to_one_b":{"b_field":10,"b_to_many_cs":[{"c_field":15}]}}}}"###
        );

        // Many items at once
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(
                data: {
                  id: 3
                  to_many_as: [
                      {
                        a_1: "a1"
                        a_2: 1
                      },
                      {
                        a_1: "a2"
                        a_2: 2
                      }
                    ]
                  to_one_b: {
                    b_to_many_cs: [
                      { c_field: 1 },
                      { c_field: 2 },
                      { c_field: 3 },
                      { c_field: 4 },
                    ]
                  }
                }
              ) {
                to_many_as {
                  a_1
                  a_2
                }
                to_one_b {
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
          "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":1},{"a_1":"a2","a_2":2}],"to_one_b":{"b_to_many_cs":[{"c_field":1},{"c_field":2},{"c_field":3},{"c_field":4}]}}}}"###
        );

        Ok(())
    }

    /// Using explicit `set` operators and shorthands mixed together, create (deeply nested) composites.
    #[connector_test]
    async fn mixed_set_create(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                to_many_as: { set: { a_1: "a1", a_2: null } }
                to_one_b: { b_field: 5 }
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
              to_one_b {
                b_field
              }
            }
          }
          "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":null}],"to_one_b":{"b_field":5}}}}"###
        );

        Ok(())
    }

    // Ensures default values are set when using an explicit set empty object
    #[connector_test]
    async fn explicit_set_empty_object(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(
                data: {
                  id: 1
                  to_many_as: { set: [{
                    a_2: null,
                  }] }
                }
              ) {
                to_many_as {
                  a_1
                  a_2
                }
              }
            }
            "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a_1 default","a_2":null}]}}}"###
        );

        // Using single-object shorthand syntax
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(
                data: {
                  id: 2
                  to_many_as: { set: {
                    a_2: null,
                  } }
                }
              ) {
                to_many_as {
                  a_1
                  a_2
                }
              }
            }
            "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a_1 default","a_2":null}]}}}"###
        );

        Ok(())
    }

    // Ensures default values are set when using a shorthand empty object
    #[connector_test]
    async fn shorthand_set_empty_object(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                to_many_as: [{
                  a_2: null,
                }]
                to_one_b: { b_to_many_cs: [{}] }
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
              to_one_b {
                b_to_many_cs {
                  c_field
                }
              }
            }
          }
        "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a_1 default","a_2":null}],"to_one_b":{"b_to_many_cs":[{"c_field":10}]}}}}"###
        );

        // Using single-object shorthand syntax
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 2
                to_many_as: [{
                  a_2: null,
                }]
                to_one_b: { b_to_many_cs: {} }
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
              to_one_b {
                b_to_many_cs {
                  c_field
                }
              }
            }
          }
        "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[{"a_1":"a_1 default","a_2":null}],"to_one_b":{"b_to_many_cs":[{"c_field":10}]}}}}"###
        );

        Ok(())
    }

    // Missing scalar lists are coerced to empty lists
    #[connector_test]
    async fn missing_lists_coerced_to_empty(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneTestModel(data: { id: 1 }) {
            to_many_as { a_1 }
            to_one_b { b_field }
          }
        }
        "#),
          @r###"{"data":{"createOneTestModel":{"to_many_as":[],"to_one_b":null}}}"###
        );

        Ok(())
    }
}
