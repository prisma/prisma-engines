use query_engine_tests::*;

// [Composites] Flavian Todo
// - Include defaults here as well (no need for separate tests, can be part of the normal tests here, see composites.rs for the commented out schema)
// - Make tests below pass where they don't.
// - Implement missing tests, suggestions:
//     - Error cases: Check that parsing correctly errors no required fields missing etc.
#[test_suite(schema(to_one_composites), only(MongoDb))]
mod create {
    /// Using explicit `set` operator, create (deeply nested) composites.
    #[connector_test]
    async fn set_create(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { set: { a_1: "a1", a_2: null } }
                b: { set: { b_field: "b_field", c: { c_field: "c_field" } } }
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null},"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}}}}"###
        );

        Ok(())
    }

    /// Using only shorthand syntax, create (deeply nested) composites.
    #[connector_test]
    async fn shorthand_set_create(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { a_1: "a1", a_2: null }
                b: { b_field: "b_field", c: { c_field: "c_field" } }
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null},"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}}}}"###
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
                b: { b_field: "b_field", c: { c_field: "c_field" } }
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null},"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}}}}"###
        );

        Ok(())
    }

    // Todo: Relies on defaults being there.

    #[connector_test]
    async fn explicit_set_empty_object(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneTestModel(
            data: {
              id: 1
              a: { set: { a_1: "a1", a_2: null } }
              b: { set: {} }
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
    async fn shorthand_set_empty_object(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { set: { a_1: "a1", a_2: null } }
                b: {}
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
}
