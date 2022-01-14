use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(to_one_composites))]
mod to_one {
    /// Using explicit `set` operators, create (deeply nested) composites.
    #[connector_test]
    async fn set_create(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: "1"
                a: { set: { a_1: "a1", a_2: null } }
                b: { set: { c: { set: {} } } }
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

    // nested set

    // shorthand create + set

    // only shorthand creates

    // set empty

    // defaults
}
