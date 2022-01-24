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
                a: { set: { a_1: "a1", a_2: null, b: { b_field: "b_field", a: [] } } }
                c: { set: [] }
              }
            ) {
              a {
                a_1
                a_2
                b {
                  b_field
                  a {
                      a_1
                  }
                }
              }
            }
          }
          "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a1","a_2":null,"b":[{"b_field":"b_field","a":[]}]}]}}}"###
        );

        // Full: set + list wrapper
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 2
                a: { set: [{ a_1: "a1", a_2: null, b: { b_field: "b_field", a: [] } }] }
                c: { set: [] }
              }
            ) {
              a {
                a_1
                a_2
                b {
                  b_field
                  a {
                      a_1
                  }
                }
              }
            }
          }
        "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a1","a_2":null,"b":[{"b_field":"b_field","a":[]}]}]}}}"###
        );

        // Many items at once
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                  createOneTestModel(
                    data: {
                      id: 3
                      a: {
                        set: [
                          {
                            a_1: "a1"
                            a_2: 2
                            b: [
                                { b_field: "b_field", a: [] },
                                { b_field: "b_field", a: [] }
                            ]
                          },
                          {
                            a_1: "a1"
                            a_2: 2
                            b: [
                                { b_field: "b_field", a: [] },
                                { b_field: "b_field", a: [] }
                            ]
                          }
                        ]
                      }
                      c: { set: [] }
                    }
                  ) {
                    a {
                      a_1
                      a_2
                      b {
                        b_field
                        a {
                            a_1
                        }
                      }
                    }
                  }
                }
              "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a1","a_2":2,"b":[{"b_field":"b_field","a":[]},{"b_field":"b_field","a":[]}]},{"a_1":"a1","a_2":2,"b":[{"b_field":"b_field","a":[]},{"b_field":"b_field","a":[]}]}]}}}"###
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
                  a: { a_1: "a1", a_2: null, b: { b_field: "b_field", a: [] } }
                  c: []
                }
              ) {
                a {
                  a_1
                  a_2
                  b {
                    b_field
                    a {
                        a_1
                    }
                  }
                }
              }
            }
            "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a1","a_2":null,"b":[{"b_field":"b_field","a":[]}]}]}}}"###
        );

        // Shorthand with explicit list wrapper.
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(
                data: {
                  id: 2
                  a: [{ a_1: "a1", a_2: null, b: { b_field: "b_field", a: [] } }]
                  c: []
                }
              ) {
                a {
                  a_1
                  a_2
                  b {
                    b_field
                    a {
                        a_1
                    }
                  }
                }
              }
            }
          "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a1","a_2":null,"b":[{"b_field":"b_field","a":[]}]}]}}}"###
        );

        // Many items at once
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                    createOneTestModel(
                      data: {
                        id: 3
                        a: [
                            {
                              a_1: "a1"
                              a_2: 2
                              b: [
                                  { b_field: "b_field", a: [] },
                                  { b_field: "b_field", a: [] }
                              ]
                            },
                            {
                              a_1: "a1"
                              a_2: 2
                              b: [
                                  { b_field: "b_field", a: [] },
                                  { b_field: "b_field", a: [] }
                              ]
                            }
                          ]
                        c: []
                      }
                    ) {
                      a {
                        a_1
                        a_2
                        b {
                          b_field
                          a {
                              a_1
                          }
                        }
                      }
                    }
                  }
                "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a1","a_2":2,"b":[{"b_field":"b_field","a":[]},{"b_field":"b_field","a":[]}]},{"a_1":"a1","a_2":2,"b":[{"b_field":"b_field","a":[]},{"b_field":"b_field","a":[]}]}]}}}"###
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
                a: { set: { a_1: "a1", a_2: null, b: [{ b_field: "b1" }] } }
                c: [{ c_field: "c1" }]
              }
            ) {
              a {
                a_1
                a_2
                b { b_field }
              }
              c {
                c_field
              }
            }
          }
          "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a1","a_2":null,"b":[{"b_field":"b1"}]}],"c":[{"c_field":"c1"}]}}}"###
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
                  a: { set: [{
                    a_2: null,
                    b: [{}]
                  }] }
                  c: { set: [] }
                }
              ) {
                a {
                  a_1
                  a_2
                  b { b_field }
                }
              }
            }
            "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a_1 default","a_2":null,"b":[{"b_field":"b_field default"}]}]}}}"###
        );

        // Using single-object shorthand syntax
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(
                data: {
                  id: 2
                  a: { set: [{
                    a_2: null,
                    b: {}
                  }] }
                  c: { set: [] }
                }
              ) {
                a {
                  a_1
                  a_2
                  b { b_field }
                }
              }
            }
            "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a_1 default","a_2":null,"b":[{"b_field":"b_field default"}]}]}}}"###
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
                a: [{
                  a_2: null,
                  b: [{}]
                }]
                c: []
              }
            ) {
              a {
                a_1
                a_2
                b { b_field }
              }
            }
          }
        "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a_1 default","a_2":null,"b":[{"b_field":"b_field default"}]}]}}}"###
        );

        // Using single-object shorthand syntax
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 2
                a: [{
                  a_2: null,
                  b: {}
                }]
                c: []
              }
            ) {
              a {
                a_1
                a_2
                b { b_field }
              }
            }
          }
        "#),
          @r###"{"data":{"createOneTestModel":{"a":[{"a_1":"a_1 default","a_2":null,"b":[{"b_field":"b_field default"}]}]}}}"###
        );

        Ok(())
    }

    // Missing scalar lists are coerced to empty lists
    #[connector_test]
    async fn missing_lists_coerced_to_empty(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneTestModel(data: { id: 1 }) {
            a { a_1 }
            c { c_field }
          }
        }
        "#),
          @r###"{"data":{"createOneTestModel":{"a":[],"c":[]}}}"###
        );

        Ok(())
    }
}
