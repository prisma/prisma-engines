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

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod update {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn update_set_explicit(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                to_many_as: { set: [{ a_1: "updated", a_2: 1337 }] }
                to_one_b: { set: { b_field: 999, b_to_many_cs: [{ c_field: 666 }] } }
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
              to_one_b {
                b_field
                b_to_many_cs { c_field }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"to_many_as":[{"a_1":"updated","a_2":1337}],"to_one_b":{"b_field":999,"b_to_many_cs":[{"c_field":666}]}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_set_shorthand(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                to_many_as: [{ a_1: "updated", a_2: 1337 }]
                to_one_b: { b_field: 999, b_to_many_cs: [{ c_field: 666 }] }
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
              to_one_b {
                b_field
                b_to_many_cs { c_field }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"to_many_as":[{"a_1":"updated","a_2":1337}],"to_one_b":{"b_field":999,"b_to_many_cs":[{"c_field":666}]}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_on_nested_update_after_a_set(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation {
          updateOneTestModel(
            where: { id: 1 }
            data: {
              to_many_as: { set: [{ a_1: "updated", a_2: { update: { increment: 3 } }, b: [] }] }
            }
          ) { id }
        }"#;

        // Ensure `update` cannot be used in the Checked type
        assert_error!(
          runner,
          query,
          2009,
          "`Mutation.updateOneTestModel.data.TestModelUpdateInput.to_many_as.CompositeAListUpdateEnvelopeInput.set.CompositeACreateInput.a_2`: Value types mismatch. Have: Object({\"update\": Object({\"increment\": Int(3)})}), want: Int"
        );

        // Ensure `update` cannot be used in the Unchecked type
        assert_error!(
          runner,
          query,
          2009,
          "`Mutation.updateOneTestModel.data.TestModelUpdateInput.to_many_as.CompositeAListUpdateEnvelopeInput.set.CompositeACreateInput.a_2`: Value types mismatch. Have: Object({\"update\": Object({\"increment\": Int(3)})}), want: Int"
        );

        Ok(())
    }

    #[connector_test]
    async fn update_push_explicit(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Test push with array & object syntax
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                to_many_as: { push: [{ a_1: "new item", a_2: 1337 }] }
                to_one_b: { upsert: {
                  set: {}
                  update: { b_to_many_cs: { push: { c_field: 111 } } }
                } }
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
          @r###"{"data":{"updateOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":null},{"a_1":"new item","a_2":1337}],"to_one_b":{"b_to_many_cs":[{"c_field":111}]}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_push_explicit_with_default(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Tests push with array & object syntax
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                to_many_as: { push: [{}] }
                to_one_b: { upsert: {
                  set: {}
                  update: { b_to_many_cs: { push: {}} }
                }}
              }
            ) {
              to_many_as {
                a_1
                a_2
              }
              to_one_b { b_to_many_cs { c_field } }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"to_many_as":[{"a_1":"a1","a_2":null},{"a_1":"a_1 default","a_2":null}],"to_one_b":{"b_to_many_cs":[{"c_field":10}]}}}}"###
        );

        Ok(())
    }

    fn mixed_composites() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              field String?
              a     A       @map("top_a")
          }

          type A {
              a_1 String @default("a_1 default") @map("a1")
              b B[]
          }

          type B {
              b_field String   @default("b_field default")
          }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(mixed_composites))]
    async fn update_push_explicit_nested(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
          id: 1
          a: { a_1: "a1", b: [{ b_field: "b_field" }] }
        }"#,
        )
        .await?;

        // Test nested push (object syntax)
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                a: { update: { b: { push: { b_field: "nested1" } } } }
              }
            ) {
              a {
                a_1
                b {
                  b_field
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","b":[{"b_field":"b_field"},{"b_field":"nested1"}]}}}}"###
        );

        // Test nested push with defaults (object syntax)
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                a: { update: { b: { push: {} } } }
              }
            ) {
              a {
                a_1
                b {
                  b_field
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","b":[{"b_field":"b_field"},{"b_field":"nested1"},{"b_field":"b_field default"}]}}}}"###
        );

        // Test nested push (array syntax)
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                a: { update: { b: { push: [{ b_field: "nested2" }, { b_field: "nested3" }] } } }
              }
            ) {
              a {
                a_1
                b {
                  b_field
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","b":[{"b_field":"b_field"},{"b_field":"nested1"},{"b_field":"b_field default"},{"b_field":"nested2"},{"b_field":"nested3"}]}}}}"###
        );

        // Test nested push with defaults (array syntax)
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                a: { update: { b: { push: [{}] } } }
              }
            ) {
              a {
                a_1
                b {
                  b_field
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","b":[{"b_field":"b_field"},{"b_field":"nested1"},{"b_field":"b_field default"},{"b_field":"nested2"},{"b_field":"nested3"},{"b_field":"b_field default"}]}}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(mixed_composites))]
    async fn fails_push_on_non_list_field(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1
              a: { a_1: "a1", b: [{ b_field: "b_field" }] }
            }"#,
        )
        .await?;

        // No push on to-one composite
        assert_error!(
            runner,
            r#"mutation {
              updateOneTestModel(
                where: { id: 1 }
                data: { a: { push: {} } }
              ) { id }
            }"#,
            2009,
            "`Mutation.updateOneTestModel.data.TestModelUpdateInput.a.ACreateInput.push`: Field does not exist on enclosing type."
        );

        // No push on scalar
        assert_error!(
          runner,
          r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: { a: { update: { a_1: { push: {} } } } }
            ) { id }
          }"#,
          2009,
          "Mutation.updateOneTestModel.data.TestModelUpdateInput.a.AUpdateEnvelopeInput.update.AUpdateInput.a_1.StringFieldUpdateOperationsInput.push`: Field does not exist on enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_unset_on_list_field(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // No unset on list fields
        assert_error!(
            runner,
            r#"mutation {
              updateOneTestModel(
                where: { id: 1 }
                data: { to_many_as: { unset: true } }
              ) { id }
            }"#,
            2009,
            "`Mutation.updateOneTestModel.data.TestModelUncheckedUpdateInput.to_many_as.CompositeACreateInput.unset`: Field does not exist on enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_upsert_on_list_field(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // No upsert on list fields
        assert_error!(
            runner,
            r#"mutation {
              updateOneTestModel(
                where: { id: 1 }
                data: { to_many_as: { upsert: {} } }
              ) { id }
            }"#,
            2009,
            "`Mutation.updateOneTestModel.data.TestModelUncheckedUpdateInput.to_many_as.CompositeACreateInput.upsert`: Field does not exist on enclosing type."
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{
                   id: 1
                   to_many_as: [{ a_1: "a1", a_2: null }]
                   to_one_b: {}
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
