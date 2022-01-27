use query_engine_tests::*;

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod create {
    use query_engine_tests::{assert_error, run_query};

    /// Using explicit `set` operator, create (deeply nested) composites.
    #[connector_test]
    async fn set_create(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { set: { a_1: "a1", a_2: null, b: { b_field: "b_field", c: { c_field: "c_field" } } } }
                b: { set: { b_field: "b_field", c: { c_field: "c_field" } } }
              }
            ) {
              a {
                a_1
                a_2
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null,"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}},"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}}}}"###
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
                a: { a_1: "a1", a_2: null, b: { b_field: "b_field", c: { c_field: "c_field" } } }
                b: { b_field: "b_field", c: { c_field: "c_field" } }
              }
            ) {
              a {
                a_1
                a_2
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null,"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}},"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}}}}"###
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
                a: { set: { a_1: "a1", a_2: null, b: { b_field: "b_field", c: { c_field: "c_field" } } } }
                b: { b_field: "b_field", c: { c_field: "c_field" } }
              }
            ) {
              a {
                a_1
                a_2
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null,"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}},"b":{"b_field":"b_field","c":{"c_field":"c_field","b":null}}}}}"###
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
              a: { set: { a_1: "a1", a_2: null, b: { c: {} } } }
              b: { set: { c: {} } }
            }
          ) {
            a {
              a_1
              a_2
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null,"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":null}}},"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":null}}}}}"###
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
                a: { a_1: "a1", a_2: null, b: { c: {} } }
                b: { c: {} }
              }
            ) {
              a {
                a_1
                a_2
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
          @r###"{"data":{"createOneTestModel":{"a":{"a_1":"a1","a_2":null,"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":null}}},"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":null}}}}}"###
        );

        Ok(())
    }

    // Fails on both the envelope and the actual input type
    #[connector_test]
    async fn fails_when_missing_required_fields(runner: Runner) -> TestResult<()> {
        // Envelope type failure
        assert_error!(
          runner,
          r#"mutation {
            createOneTestModel(
              data: {
                id: 1
                a: { set: { a_1: "a1", a_2: null, b: { c: {} } } }
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
                a: { set: { a_1: "a1", a_2: null, b: { c: {} } } }
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

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod update {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn update_set_envelope(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Update set on required composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { set: { a_1: "a_updated", a_2: 1337, b: { b_field: "b_updated", c: { c_field: "c_updated" } } } } }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_updated","a_2":1337,"b":{"b_field":"b_updated","c":{"c_field":"c_updated"}}},"b":{"b_field":"b1","c":{"c_field":"c1"}}}}}"###
        );

        // Update set on optional composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { b: { set: { c: { c_field: "updated" } } } }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_updated","a_2":1337,"b":{"b_field":"b_updated","c":{"c_field":"c_updated"}}},"b":{"b_field":"b_field default","c":{"c_field":"updated"}}}}}"###
        );

        // Nested empty object with defaults
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: {
              a: { set: { b: { c: {} } } }
              b: { set: { c: {} } }
            }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_1 default","a_2":null,"b":{"b_field":"b_field default","c":{"c_field":"c_field default"}}},"b":{"b_field":"b_field default","c":{"c_field":"c_field default"}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_set_shorthand(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Update set on required composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { a_1: "a_updated", a_2: 1337, b: { b_field: "b_updated", c: { c_field: "c_updated" } } } }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_updated","a_2":1337,"b":{"b_field":"b_updated","c":{"c_field":"c_updated"}}},"b":{"b_field":"b1","c":{"c_field":"c1"}}}}}"###
        );

        // Update set on optional composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { b: { c: { c_field: "updated" } } }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_updated","a_2":1337,"b":{"b_field":"b_updated","c":{"c_field":"c_updated"}}},"b":{"b_field":"b_field default","c":{"c_field":"updated"}}}}}"###
        );

        // Nested empty object with defaults
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: {
              a: { b: { c: {} } }
              b: { c: {} }
            }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_1 default","a_2":null,"b":{"b_field":"b_field default","c":{"c_field":"c_field default"}}},"b":{"b_field":"b_field default","c":{"c_field":"c_field default"}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_set_mixed(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Top-level
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: {
              a: { set: { a_2: 1337, b: { b_field: "b_updated", c: { c_field: "c_updated" } } } },
              b: { c: { c_field: "c_updated" } }
            }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_1 default","a_2":1337,"b":{"b_field":"b_updated","c":{"c_field":"c_updated"}}},"b":{"b_field":"b_field default","c":{"c_field":"c_updated"}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_nested_envelope(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Top-level
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { update: {
              a_2: { increment: 1335 },
            }}}
          ){
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":1337,"b":{"b_field":"b1","c":{"c_field":"c1"}}},"b":{"b_field":"b1","c":{"c_field":"c1"}}}}}"###
        );

        // Nested
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: {
              a: {
                update: {
                  a_2: { decrement: 1 }
                  b: {
                    update: {
                      b_field: "b_updated"
                      c: { update: { c_field: "c_updated" } }
                    }
                  }
                }
              }
            }
          ){
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
            b { b_field c { c_field } }
          } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":1336,"b":{"b_field":"b_updated","c":{"c_field":"c_updated"}}},"b":{"b_field":"b1","c":{"c_field":"c1"}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn mixed_update_set(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: {
              a: {
                update: {
                  b: { update: { c: {} } }
                }
              }
            }
          ) {
            a {
              a_1 a_2
              b { b_field c { c_field } }
            }
          }}"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":2,"b":{"b_field":"b1","c":{"c_field":"c_field default"}}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_unset_explicit(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Nested scalar
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { b: { update: { c: { update: { c_opt: { unset: true } } } } } }
          ) { a { a_1 a_2 } b { b_field c { c_opt c_field } } } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":2},"b":{"b_field":"b1","c":{"c_opt":null,"c_field":"c1"}}}}}"###
        );

        // Top-level scalar
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { update: { a_2: { unset: true } } } }
          ) { a { a_1 a_2 } b { b_field c { c_field } } } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":null},"b":{"b_field":"b1","c":{"c_field":"c1"}}}}}"###
        );

        // Top-level composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { b: { unset: true } }
          ) { a { a_1 a_2 } b { b_field c { c_field } } } }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":null},"b":null}}}"###
        );

        Ok(())
    }

    // Ensures unset is only available in composite types of type:
    // - Scalar
    // - Composite
    #[connector_test]
    async fn ensure_unset_unavailable_on_fields(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"mutation { updateOneTestModel(
              where: { id: 1 },
              data: { field: { unset: true } }
            ) { id }}"#,
            2009,
            "Mutation.updateOneTestModel.data.TestModelUpdateInput.field.NullableStringFieldUpdateOperationsInput.unset`: Field does not exist on enclosing type"
        );

        assert_error!(
          runner,
          r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { update: { a_1: { unset: true } } } }
          ) { id }}"#,
          2009,
          "`Mutation.updateOneTestModel.data.TestModelUncheckedUpdateInput.a.AUpdateEnvelopeInput.update.AUpdateInput.a_1.StringFieldUpdateOperationsInput.unset`: Field does not exist on enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_on_nested_update_after_a_set(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
          runner,
          r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                b: { set: { c: { update: { c_field: "updated" } } } }
              }
            ) {
              id
            }
          }"#,
          2009,
          "`Mutation.updateOneTestModel.data.TestModelUpdateInput.b.BNullableUpdateEnvelopeInput.set.BCreateInput.c.CCreateInput.update`: Field does not exist on enclosing type."
        );

        assert_error!(
          runner,
          r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                b: { c: { update: { c_field: "updated" } } }
              }
            ) {
              id
            }
          }"#,
          2009,
          "`Mutation.updateOneTestModel.data.TestModelUpdateInput.b.BCreateInput.c.CCreateInput.update`: Field does not exist on enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_when_missing_required_fields(runner: Runner) -> TestResult<()> {
        // Envelope type failure
        assert_error!(
              runner,
              r#"mutation {
                updateOneTestModel(
                  where: { id: 1 }
                  data: {
                    b: {}
                  }
                ) {
                  id
                }
              }"#,
              2009,
              "`Mutation.updateOneTestModel.data.TestModelUpdateInput.b.BNullableUpdateEnvelopeInput`: Expected exactly one field to be present, got 0."
            );

        // Missing required field without default failure on field `B.c`
        assert_error!(
                runner,
                r#"mutation {
                updateOneTestModel(
                  where: { id: 1 }
                  data: {
                    b: {}
                  }
                ) {
                  id
                }
              }"#,
                2009,
                "`Mutation.updateOneTestModel.data.TestModelUpdateInput.b.BCreateInput.c`: A value is required but not set."
            );

        Ok(())
    }

    #[connector_test]
    async fn fails_update_on_optional_composite(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Invalid `update` on optional composite field
        assert_error!(
          runner,
          r#"mutation {
          updateOneTestModel(
            where: { id: 1 }
            data: {
              b: { update: { b_field: "b_updated" } }
            }
          ) {
            id
          }
        }"#,
          2009,
          "`Mutation.updateOneTestModel.data.TestModelUpdateInput.b.BNullableUpdateEnvelopeInput.update`: Field does not exist on enclosing type."
      );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{
          id: 1
          a: { set: { a_1: "a1", a_2: 2, b: { b_field: "b1", c: { c_field: "c1" } } } }
          b: { set: { b_field: "b1", c: { c_field: "c1" } } }
        }"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
