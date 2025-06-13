use query_engine_tests::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod create_list {
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
          @r###"{"data":{"createOneTestModel":{"to_many_as":null,"to_one_b":null}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod update_list {
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
        assert_error!(runner, query, 2009, "Invalid argument type");

        // Ensure `update` cannot be used in the Unchecked type
        assert_error!(runner, query, 2009, "Invalid argument type");

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
    async fn update_push_with_dollar_string(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Test push with array & object syntax
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                to_many_as: { push: [{ a_1: "$new_item" }] }
              }
            ) {
              to_many_as {
                a_1
              }
            }
          }
          "#),
          @r###"{"data":{"updateOneTestModel":{"to_many_as":[{"a_1":"a1"},{"a_1":"$new_item"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                to_many_as: { push: { a_1: "$new_item_2" } }
              }
            ) {
              to_many_as {
                a_1
              }
            }
          }
          "#),
          @r###"{"data":{"updateOneTestModel":{"to_many_as":[{"a_1":"a1"},{"a_1":"$new_item"},{"a_1":"$new_item_2"}]}}}"###
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

    fn mixed_to_one_to_many() -> String {
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

    #[connector_test(schema(mixed_to_one_to_many))]
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

    #[connector_test(schema(mixed_to_one_to_many))]
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
            "Field does not exist in enclosing type."
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
            "Field does not exist in enclosing type."
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
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_upsert_on_list_field(runner: Runner) -> TestResult<()> {
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
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn update_many_simple(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
               id: 1
               to_many_as: [
                 {
                   a_1: "a1_new",
                   a_2: 0,
                 }
               ]
             }"#,
        )
        .await?;

        // `set` within `updateMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_1: "a1_new" },
                  data: {
                    a_1: { set: "a1_updated" },
                    a_2: { set: 1 },
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_1
                a_2
                a_to_one_b {
                  b_field
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_1":"a1_updated","a_2":1,"a_to_one_b":null}]}}}"###
        );

        // `upsert` within `updateMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_1: "a1_updated" },
                  data: {
                    a_to_one_b: {
                      upsert: {
                        set: { b_field: 0 },
                        update: {
                          b_field: 1
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_1
                a_2
                a_to_one_b {
                  b_field
                  b_to_one_c {
                    c_field
                    c_to_many_as {
                      a_1
                      a_2
                    }
                  }
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_1":"a1_updated","a_2":1,"a_to_one_b":{"b_field":0,"b_to_one_c":null,"b_to_many_cs":[]}}]}}}"###
        );

        // numeric updates (update & upsert) within `updateMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_to_one_b: { is: { b_field: 0 } } },
                  data: {
                    a_2: { increment: 1 },
                    a_to_one_b: {
                      upsert: {
                        set: { b_field: 0 },
                        update: {
                          b_field: { increment: 1 }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_1
                a_2
                a_to_one_b {
                  b_field
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_1":"a1_updated","a_2":2,"a_to_one_b":{"b_field":1}}]}}}"###
        );

        // `push` within `updateMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_to_one_b: { is: { b_field: 1 } } },
                  data: {
                    a_to_one_b: {
                      upsert: {
                        set: { b_field: 0 },
                        update: {
                          b_to_many_cs: {
                            push: [{ c_field: 1 }, { c_field: 1 }]
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_to_one_b {
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_to_one_b":{"b_to_many_cs":[{"c_field":1},{"c_field":1}]}}]}}}"###
        );

        // `updateMany` within `updateMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_to_one_b: { is: { b_to_many_cs: { every: { c_field: 1 } } } } },
                  data: {
                    a_to_one_b: {
                      upsert: {
                        set: { b_field: 0 },
                        update: {
                          b_to_many_cs: {
                            updateMany: {
                              where: { c_field: 1 },
                              data: { c_field: { multiply: 2 } }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_to_one_b {
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_to_one_b":{"b_to_many_cs":[{"c_field":2},{"c_field":2}]}}]}}}"###
        );

        // `deleteMany` within `updateMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_to_one_b: { is: { b_to_many_cs: {  every: { c_field: 2 } } } } },
                  data: {
                    a_to_one_b: {
                      upsert: {
                        set: { b_field: 0 },
                        update: {
                          b_to_many_cs: {
                            deleteMany: { where: { c_field: 2 } }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_to_one_b {
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_to_one_b":{"b_to_many_cs":[]}}]}}}"###
        );

        // `unset` within `updateMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_1: "a1_updated" },
                  data: {
                    a_to_one_b: { unset: true }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_to_one_b {
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_to_one_b":null}]}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_many_in_upsert(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
               id: 1
               to_many_as: []
             }"#,
        )
        .await?;

        let query = r#"mutation {
          updateOneTestModel(where: { id: 1 }, data: {
            to_one_b: {
              upsert: {
                set: { b_to_many_cs: [{ c_field: 0 }, { c_field: 1 }, { c_field: 2 }] },
                update: {
                  b_to_many_cs: {
                    updateMany: {
                      where: { c_field: { gt: 0 } },
                      data: {
                        c_field: { multiply: 2 }
                      }
                    }
                  }
                }
              }
            }
          }) {
            id
            to_one_b {
              b_to_many_cs {
                c_field
              }
            }
          }
        }"#;

        // set
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_one_b":{"b_to_many_cs":[{"c_field":0},{"c_field":1},{"c_field":2}]}}}}"###
        );

        // update
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_one_b":{"b_to_many_cs":[{"c_field":0},{"c_field":2},{"c_field":4}]}}}}"###
        );

        Ok(())
    }

    fn to_many_nested_to_one() -> String {
        let schema = indoc! {
            r#"model TestModel {
            #id(id, Int, @id)
            to_many_as CompositeA[] @map("top_a")
        }

        type CompositeA {
            a_field    Int        @map("a_int")
            a_to_one_b CompositeB @map("to_one_b")
        }

        type CompositeB {
          b_field        Int         @map("b_int") @default(0)
          b_to_one_c     CompositeC  @map("to_one_c")
          b_to_one_c_opt CompositeC? @map("to_one_c_opt")
        }

        type CompositeC {
          c_field        Int         @map("c_int") @default(0)
          c_opt_field    Int?        @map("c_opt_int") @default(0)
          c_to_one_d_opt CompositeD? @map("to_one_d")
          c_to_one_e_opt CompositeE? @map("to_one_e")
        }

        type CompositeD {
          d_field Int @map("d_int") @default(0)
        }

        type CompositeE {
          e_field Int @map("e_int") @default(0)
        }
        "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(to_many_nested_to_one))]
    async fn update_many_with_nested_updates(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
          id: 1,
          to_many_as: [
            {
              a_field: 0,
              a_to_one_b: {
                b_field: 0,
                b_to_one_c: {
                  c_field: 0,
                  c_to_one_d_opt: { d_field: 1 },
                  c_to_one_e_opt: { e_field: 1 },
                },
                b_to_one_c_opt: {
                  c_field: 0,
                  c_to_one_d_opt: { d_field: 1 },
                  c_to_one_e_opt: { e_field: 1 },
                },
              },
            }
          ]
        }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_field: 0 },
                  data: {
                    a_field: 1,
                    a_to_one_b: {
                      update: {
                        b_field: 1,
                        b_to_one_c: {
                          update: {
                            c_field: 1
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_field
                a_to_one_b {
                  b_field
                  b_to_one_c {
                    c_field
                    c_to_one_d_opt { d_field }
                    c_to_one_e_opt { e_field }
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_field":1,"a_to_one_b":{"b_field":1,"b_to_one_c":{"c_field":1,"c_to_one_d_opt":{"d_field":1},"c_to_one_e_opt":{"e_field":1}}}}]}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(to_many_nested_to_one))]
    async fn update_many_with_nested_unsets(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1,
              to_many_as: [
                {
                  a_field: 0,
                  a_to_one_b: {
                    b_field: 0,
                    b_to_one_c: {
                      c_field: 0,
                      c_to_one_d_opt: { d_field: 1 },
                      c_to_one_e_opt: { e_field: 1 },
                    },
                    b_to_one_c_opt: {
                      c_field: 0,
                      c_to_one_d_opt: { d_field: 1 },
                      c_to_one_e_opt: { e_field: 1 },
                    },
                  },
                }
              ]
            }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{
            id: 2,
            to_many_as: [
              {
                a_field: 0,
                a_to_one_b: {
                  b_field: 0,
                  b_to_one_c: {
                    c_field: 1,
                    c_to_one_d_opt: { d_field: 1 },
                    c_to_one_e_opt: { e_field: 1 },
                  },
                  b_to_one_c_opt: {
                    c_field: 0,
                    c_to_one_d_opt: { d_field: 1 },
                    c_to_one_e_opt: { e_field: 1 },
                  },
                },
              }
            ]
          }"#,
        )
        .await?;

        // (nested multiple unsets + scalar update) within updateMany
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_field: 0 },
                  data: {
                    a_field: 1,
                    a_to_one_b: {
                      update: {
                        b_field: 1,
                        b_to_one_c: {
                          update: {
                            c_field: 1,
                            c_opt_field: { unset: true }
                            c_to_one_d_opt: { unset: true },
                            c_to_one_e_opt: { unset: true },
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_field
                a_to_one_b {
                  b_field
                  b_to_one_c {
                    c_field
                    c_opt_field
                    c_to_one_d_opt { d_field }
                    c_to_one_e_opt { e_field }
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_field":1,"a_to_one_b":{"b_field":1,"b_to_one_c":{"c_field":1,"c_opt_field":null,"c_to_one_d_opt":null,"c_to_one_e_opt":null}}}]}}}"###
        );

        // (nested multiple unsets without any other updates) within updateMany
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 2 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_field: 0 },
                  data: {
                    a_to_one_b: {
                      update: {
                        b_to_one_c: {
                          update: {
                            c_opt_field: { unset: true }
                            c_to_one_d_opt: { unset: true },
                            c_to_one_e_opt: { unset: true },
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_field
                a_to_one_b {
                  b_field
                  b_to_one_c {
                    c_field
                    c_opt_field
                    c_to_one_d_opt { d_field }
                    c_to_one_e_opt { e_field }
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":2,"to_many_as":[{"a_field":0,"a_to_one_b":{"b_field":0,"b_to_one_c":{"c_field":1,"c_opt_field":null,"c_to_one_d_opt":null,"c_to_one_e_opt":null}}}]}}}"###
        );

        // (nested multiple unsets + scalar updates) within upsert within updateMany
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_field: 1 },
                  data: {
                    a_field: 2,
                    a_to_one_b: {
                      update: {
                        b_field: 2,
                        b_to_one_c_opt: {
                          upsert: {
                            set: { c_field: 0 },
                            update: {
                              c_field: 2,
                              c_opt_field: { unset: true }
                              c_to_one_d_opt: { unset: true },
                              c_to_one_e_opt: { unset: true },
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_field
                a_to_one_b {
                  b_field
                  b_to_one_c_opt {
                    c_field
                    c_opt_field
                    c_to_one_d_opt { d_field }
                    c_to_one_e_opt { e_field }
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_field":2,"a_to_one_b":{"b_field":2,"b_to_one_c_opt":{"c_field":2,"c_opt_field":null,"c_to_one_d_opt":null,"c_to_one_e_opt":null}}}]}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(to_many_nested_to_one))]
    async fn update_many_with_nested_upserts(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
          id: 1,
          to_many_as: [
            {
              a_field: 0,
              a_to_one_b: {
                b_field: 0,
                b_to_one_c: {
                  c_field: 0,
                  c_to_one_d_opt: { d_field: 0 },
                  c_to_one_e_opt: { e_field: 0 },
                },
                b_to_one_c_opt: {
                  c_field: 0,
                  c_to_one_d_opt: { d_field: 0 },
                  c_to_one_e_opt: { e_field: 0 },
                },
              },
            }
          ]
        }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                updateMany: {
                  where: { a_field: 0 },
                  data: {
                    a_field: 1,
                    a_to_one_b: {
                      update: {
                        b_field: 1,
                        b_to_one_c_opt: {
                          upsert: {
                            set: { c_field: 0 },
                            update: {
                              c_field: 1,
                              c_to_one_d_opt: {
                                upsert: {
                                  set: { d_field: 0 },
                                  update: {
                                    d_field: 1
                                  }
                                }
                              },
                              c_to_one_e_opt: {
                                upsert: {
                                  set: { e_field: 0 },
                                  update: {
                                    e_field: 1
                                  }
                                }
                              },
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_field
                a_to_one_b {
                  b_field
                  b_to_one_c_opt {
                    c_field
                    c_to_one_d_opt { d_field }
                    c_to_one_e_opt { e_field }
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_field":1,"a_to_one_b":{"b_field":1,"b_to_one_c_opt":{"c_field":1,"c_to_one_d_opt":{"d_field":1},"c_to_one_e_opt":{"e_field":1}}}}]}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_many_complex(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
               id: 1
               to_many_as: [
                 {
                   a_1: "a1_new",
                   a_2: 0,
                 }
               ]
               to_one_b: {
                 b_field: 1,
                 b_to_many_cs: [
                   {
                     c_field: 1,
                     c_to_many_as: [
                       { a_1: "a1_new", a_2: 0 }
                     ]
                    }
                 ]
               }
             }"#,
        )
        .await?;

        // Tests:
        // Nested updateMany within upsert
        // updateMany with: set, push, numeric updates...
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                to_one_b: {
                  upsert: {
                    set: {
                      b_field: 0
                      b_to_many_cs: { c_field: 0, c_to_many_as: { a_1: "a1_new", a_2: 0 } }
                    }
                    update: {
                      b_field: { multiply: 3 }
                      b_to_many_cs: {
                        updateMany: {
                          where: { c_field: 1, c_to_many_as: { some: { a_1: "a1_new" } } }
                          data: {
                            c_field: { decrement: 1 }
                            c_to_many_as: {
                              push: { a_1: "a_1_pushed", a_2: 2 }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            ) {
              id
              to_one_b {
                b_field
                b_to_many_cs {
                  c_field
                  c_to_many_as {
                    a_1
                    a_2
                  }
                }
              }
            }
          }
          "#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_one_b":{"b_field":3,"b_to_many_cs":[{"c_field":0,"c_to_many_as":[{"a_1":"a1_new","a_2":0},{"a_1":"a_1_pushed","a_2":2}]}]}}}}"###
        );

        // Tests:
        // Top-level updateMany
        // Nested upsert within updateMany
        // Nested updateMany within upsert
        // Nested updateMany within updateMany
        // Nested unset within updateMany
        let query = r#"mutation {
          updateOneTestModel(
            where: { id: 1 }
            data: {
              to_many_as: {
                updateMany: {
                  where: { a_2: { gte: 0 } }
                  data: {
                    a_1: { set: "a1_updated" }
                    a_2: { increment: 1 }
                    a_to_one_b: {
                      upsert: {
                        set: {
                          b_field: 0
                          b_to_many_cs: [{ c_field: 0, c_to_many_as: [{ a_1: "a1_new", a_2: 0, a_to_one_b: { b_field: 0 } }] }]
                        }
                        update: {
                          b_field: { increment: 1 }
                          b_to_many_cs: {
                            updateMany: {
                              where: { c_to_many_as: { none: { a_2: 10 } } }
                              data: {
                                c_field: { increment: 2 },
                                c_to_many_as: {
                                  updateMany: {
                                    where: { a_2: { gte: 0 } },
                                    data: {
                                      a_1: "a1_updated",
                                      a_2: { set: 1337 },
                                      a_to_one_b: { unset: true }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          ) {
            id
            to_many_as {
              a_1
              a_2
              a_to_one_b {
                b_field
                b_to_many_cs {
                  c_field
                  c_to_many_as {
                    a_1
                    a_2
                    a_to_one_b { b_field }
                  }
                }
              }
            }
          }
        }          
        "#;

        // upsert set
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_1":"a1_updated","a_2":1,"a_to_one_b":{"b_field":0,"b_to_many_cs":[{"c_field":0,"c_to_many_as":[{"a_1":"a1_new","a_2":0,"a_to_one_b":{"b_field":0}}]}]}}]}}}"###
        );

        // upsert update
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_1":"a1_updated","a_2":2,"a_to_one_b":{"b_field":1,"b_to_many_cs":[{"c_field":2,"c_to_many_as":[{"a_1":"a1_updated","a_2":1337,"a_to_one_b":null}]}]}}]}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn delete_many_explicit(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                  id: 1
                  to_many_as: [
                    { a_1: "a_1", a_2: 0, a_to_one_b: { b_field: 0, b_to_many_cs: [{ c_field: 0 }, { c_field: 0 }] } },
                    { a_1: "a_2", a_2: 1, a_to_one_b: { b_field: 1, b_to_many_cs: [{ c_field: 1 }, { c_field: 2 }] } },
                    { a_1: "a_3", a_2: 2, a_to_one_b: { b_field: 2, b_to_many_cs: [{ c_field: 0 }, { c_field: 0 }] } },
                  ],
                }"#,
        )
        .await?;

        // Top-level `deleteMany`
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              to_many_as: {
                deleteMany: {
                  where: {
                    a_to_one_b: {
                      is: {
                        b_field: { not: 1 }
                        b_to_many_cs: {
                          every: { c_field: 0 }
                        }
                      }
                    }
                  }
                }
              }
            }) {
              id
              to_many_as {
                a_1
                a_2
                a_to_one_b {
                  b_field
                  b_to_many_cs {
                    c_field
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_many_as":[{"a_1":"a_2","a_2":1,"a_to_one_b":{"b_field":1,"b_to_many_cs":[{"c_field":1},{"c_field":2}]}}]}}}"###
        );

        let query = r#"mutation {
          updateOneTestModel(where: { id: 1 }, data: {
            to_one_b: {
              upsert: {
                set: {
                  b_to_many_cs: [{ c_field: 0 }, { c_field: 1 }, { c_field: 3 }]
                },
                update: {
                  b_to_many_cs: {
                    deleteMany: { where: { c_field: { gt: 0 } } }
                  }
                }
              }
            }
          }) {
            id
            to_one_b {
              b_to_many_cs {
                c_field
              }
            }
          }
        }"#;

        // `deleteMany` within `upsert` (set)
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_one_b":{"b_to_many_cs":[{"c_field":0},{"c_field":1},{"c_field":3}]}}}}"###
        );

        // `deleteMany` within `upsert` (update)
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"id":1,"to_one_b":{"b_to_many_cs":[{"c_field":0}]}}}}"###
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
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
