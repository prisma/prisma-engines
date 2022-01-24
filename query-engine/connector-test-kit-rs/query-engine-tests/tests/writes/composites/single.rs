use query_engine_tests::*;

#[test_suite(only(MongoDb))]
mod create {
    use query_engine_tests::{assert_error, run_query};

    /// Using explicit `set` operator, create (deeply nested) composites.
    #[connector_test(schema(to_one_composites))]
    async fn set_create_to_one(runner: Runner) -> TestResult<()> {
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
    #[connector_test(schema(to_one_composites))]
    async fn shorthand_set_create_to_one(runner: Runner) -> TestResult<()> {
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
    #[connector_test(schema(to_one_composites))]
    async fn mixed_set_create_to_one(runner: Runner) -> TestResult<()> {
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

    // Ensures default values are set when using an explicit set empty object
    #[connector_test(schema(to_one_composites))]
    async fn explicit_set_empty_object_to_one(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneTestModel(
            data: {
              id: 1
              a: { set: { a_1: "a1", a_2: null } }
              b: { set: { c: {} } }
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null},"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":null}}}}}"###
        );

        Ok(())
    }

    // Ensures default values are set when using a shorthand empty object
    #[connector_test(schema(to_one_composites))]
    async fn shorthand_set_empty_object_to_one(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { set: { a_1: "a1", a_2: null } }
                b: { c: {} }
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null},"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":null}}}}}"###
        );

        Ok(())
    }

    // Fails on both the envelope and the actual input type
    #[connector_test(schema(to_one_composites))]
    async fn error_when_missing_required_fields(runner: Runner) -> TestResult<()> {
        // Envelope type failure
        assert_error!(
          runner,
          r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { set: { a_1: "a1", a_2: null } }
                b: {}
              }
            ) {
              id
            }
          }"#,
          2009,
          "Mutation.createOneTestModel.data.TestModelUncheckedCreateInput.b.BCreateEnvelopeInput`: Expected exactly one field to be present, got 0."
        );

        // Missing required field without default failure on field `B.c`
        assert_error!(
            runner,
            r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { set: { a_1: "a1", a_2: null } }
                b: {}
              }
            ) {
              id
            }
          }"#,
            2009,
            "Mutation.createOneTestModel.data.TestModelCreateInput.b.BCreateInput.c`: A value is required but not set."
        );

        Ok(())
    }
}
