use indoc::indoc;
use query_engine_tests::*;

// [Composites] Flavian Todo
// - Include defaults here as well (no need for separate tests, can be part of the normal tests here, see composites.rs for the commented out schema)
// - Make tests below pass where they don't.
// - Implement missing tests, suggestions:
//     - Error cases: Check that parsing correctly errors no required fields missing etc.
#[test_suite(schema(to_many_composites))]
mod create {
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
                a: { set: { a_1: "a1", a_2: null } }
                b: { b_field: "b_field", c: { set: { c_field: "c_field" } } }
              }
            ) {
              a {
                a_1
                a_2
              }
              b {
                b_field
                c {
                  c_field
                  b {
                    b_field
                  }
                }
              }
            }
          }
          "#),
          @r###""###
        );

        Ok(())
    }

    // Todo: Relies on defaults being there.
    #[connector_test]
    async fn explicit_set_empty_object(runner: Runner) -> TestResult<()> {
        todo!()
    }

    // Todo: Relies on defaults being there.
    #[connector_test]
    async fn shorthand_set_empty_object(runner: Runner) -> TestResult<()> {
        todo!()
    }
}
