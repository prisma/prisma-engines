use query_engine_tests::*;

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod create_single {
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
            "Expected exactly one field to be present, got 0."
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
            "A value is required but not set"
        );

        Ok(())
    }
}

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod update_single {
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
        create_row(
            &runner,
            r#"{
          id: 1
          field: "1",
          a: { b: { c: { c_opt: "nested_1", b: { c: {}} } } }
          b: { c: {} }
        }"#,
        )
        .await?;

        // Nested composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { update: { b: { update: { c: { update: { b: { unset: true } } } } } } } }
          ) {
            a {
              a_1 a_2
              b { c { b { b_field } } }
            }
          }}"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_1 default","a_2":null,"b":{"c":{"b":null}}}}}}"###
        );

        // Top-level composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { b: { unset: true } }
          ) {
            a {
              a_1 a_2
              b { c { b { b_field } } }
            }
            b {
              b_field
            }
          }}"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_1 default","a_2":null,"b":{"c":{"b":null}}},"b":null}}}"###
        );

        // Nested scalar
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                a: {
                  update: { b: { update: { c: { update: { c_opt: { unset: true } } } } } }
                }
              }
            ) {
              a {
                a_1
                a_2
                b {
                  c {
                    c_opt
                  }
                }
              }
            }
          }
          "#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a_1 default","a_2":null,"b":{"c":{"c_opt":null}}}}}}"###
        );

        // Top-level scalar
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: { field: { unset: true } }) {
              field
              a {
                a_1
                a_2
                b {
                  c {
                    c_opt
                  }
                }
              }
            }
          }
          "#),
          @r###"{"data":{"updateOneTestModel":{"field":null,"a":{"a_1":"a_1 default","a_2":null,"b":{"c":{"c_opt":null}}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_unset_false_is_noop(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Top-level composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { b: { unset: false } }
          ) {
            b {
              b_field
            }
          }}"#),
          @r###"{"data":{"updateOneTestModel":{"b":{"b_field":"b1"}}}}"###
        );

        // Optional scalar
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { update: { a_2: { unset: false } } } }
          ) {
            a {
              a_2
            }
          }}"#),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_2":2}}}}"###
        );

        Ok(())
    }

    // Ensures unset is only available on optional scalars/to-one composite
    #[connector_test]
    async fn ensure_unset_unavailable_on_fields(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"mutation { updateOneTestModel(
            where: { id: 1 },
            data: { a: { update: { a_1: { unset: true } } } }
          ) { id }}"#,
            2009,
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn update_upsert_explicit(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1
              a: { set: { a_1: "a1", a_2: 2, b: { c: {} } } }
            }"#,
        )
        .await?;

        let nested_query = r#"mutation { updateOneTestModel(
          where: { id: 1 },
          data: {
            a: { update: { b: { update: { c: { update: { b: {
              upsert: {
                set: { b_field: "new", c: { c_field: "new" } },
                update: {
                  b_field: "updated",
                  c: {
                    update: {
                      c_field: "updated"
                      b: {
                        upsert: {
                          set: { b_field: "new", c: { c_field: "new" } },
                          update: {
                            b_field: "updated",
                          }
                        }
                      }
                    }
                  }
                }
              }
            } } } } } } }
          }
        ) {
          a {
            a_1 a_2
            b {
              b_field
              c {
                c_field
                b {
                  b_field
                  c {
                    c_field
                    b { b_field }
                  }
                }
              }
            }
          }
        }}"#;

        // Nested composite - upsert set
        insta::assert_snapshot!(
          run_query!(&runner, nested_query),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":2,"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":{"b_field":"new","c":{"c_field":"new","b":{"b_field":"new"}}}}}}}}}"###
        );

        // Nested composite - upsert update
        insta::assert_snapshot!(
          run_query!(&runner, nested_query),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":2,"b":{"b_field":"b_field default","c":{"c_field":"c_field default","b":{"b_field":"updated","c":{"c_field":"updated","b":{"b_field":"updated"}}}}}}}}}"###
        );

        let top_level_query = r#"mutation { updateOneTestModel(
          where: { id: 1 },
          data: { b: { upsert: {
            set: { b_field: "new", c: { c_field: "new" } },
            update: { b_field: "updated", c: { c_field: "updated" } }
          } } }
        ) { a { a_1 a_2 } b { b_field c { c_field } } } }"#;

        // Top-level composite
        insta::assert_snapshot!(
          run_query!(&runner, top_level_query),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":2},"b":{"b_field":"new","c":{"c_field":"new"}}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, top_level_query),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a1","a_2":2},"b":{"b_field":"updated","c":{"c_field":"updated"}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn mixed_upsert_update_set_unset(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1
              a: { set: { a_1: "a1", a_2: 2, b: { c: {} } } }
            }"#,
        )
        .await?;

        let query = r#"mutation {
          updateOneTestModel(
            where: { id: 1 }
            data: {
              a: {
                update: {
                  a_1: { set: "a.a_1.updated" }
                  a_2: { increment: 1335 }
                  b: {
                    update: {
                      b_field: "a.b.b_field.updated"
                      c: {
                        update: {
                          c_field: "a.b.c.c_field.updated"
                          b: {
                            upsert: {
                              set: { b_field: "a.b.c.b.b_field.new", c: { c_field: "a.b.c.b.c.c_field.new", b: { c: {} } } }
                              update: {
                                b_field: { set: "a.b.c.b.b_field.updated" }
                                c: { update: { b: { unset: true } } }
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
            a {
              a_1
              a_2
              b {
                b_field
                c {
                  c_field
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
            }
          }
        }
        "#;

        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a.a_1.updated","a_2":1337,"b":{"b_field":"a.b.b_field.updated","c":{"c_field":"a.b.c.c_field.updated","b":{"b_field":"a.b.c.b.b_field.new","c":{"c_field":"a.b.c.b.c.c_field.new","b":{"b_field":"b_field default"}}}}}}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"updateOneTestModel":{"a":{"a_1":"a.a_1.updated","a_2":2672,"b":{"b_field":"a.b.b_field.updated","c":{"c_field":"a.b.c.c_field.updated","b":{"b_field":"a.b.c.b.b_field.updated","c":{"c_field":"a.b.c.b.c.c_field.new","b":null}}}}}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_unset_on_required_field(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"mutation { updateOneTestModel(
              where: { id: 1 },
              data: { a: { unset: true } }
            ) { id }}"#,
            2009,
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_upsert_on_required_field(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"mutation { updateOneTestModel(
              where: { id: 1 },
              data: { a: { upsert: { set: {}, update: {} } } }
            ) { id }}"#,
            2009,
            "Field does not exist in enclosing type."
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
            "Field does not exist in enclosing type."
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
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_set_when_missing_required_fields(runner: Runner) -> TestResult<()> {
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
            "Expected exactly one field to be present, got 0."
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
            "A value is required but not set"
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
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_update_many_on_to_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Fails on required to-one
        assert_error!(
            runner,
            r#"mutation {
              updateOneTestModel(where: { id: 1 }, data: {
                a: { updateMany: {} }
              }) {
                id
              }
            }"#,
            2009,
            "Field does not exist in enclosing type."
        );

        // Fails on optional to-one
        assert_error!(
            runner,
            r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              b: { updateMany: {} }
            }) {
              id
            }
          }"#,
            2009,
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_delete_many_on_to_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Fails on required to-one
        assert_error!(
            runner,
            r#"mutation {
            updateOneTestModel(where: { id: 1 }, data: {
              a: { deleteMany: {} }
            }) {
              id
            }
          }"#,
            2009,
            "Field does not exist in enclosing type."
        );

        // Fails on optional to-one
        assert_error!(
            runner,
            r#"mutation {
          updateOneTestModel(where: { id: 1 }, data: {
            b: { deleteMany: {} }
          }) {
            id
          }
        }"#,
            2009,
            "Field does not exist in enclosing type."
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
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
