use query_engine_tests::*;

#[test_suite]
mod delete_inside_upsert {
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams};
    use query_test_macros::relation_link_test;

    // "a P1 to C1  relation " should "work through a nested mutation by id"
    // TODO:(dom): Not working on mongo. Failing from 9-17
    // Reason: Misses foreign key cascade emulation for update
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt", exclude(MongoDb))]
    async fn p1_c1_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childOpt: {{
                          create: {{c: "c1", c_1: "foo", c_2: "bar"}}
                        }}
                      }}){{
                        {parent_selection}
                        childOpt{{
                          {child_selection}
                        }}
                      }}
                    }}"#,
                    parent_selection = t.parent().selection(),
                    child_selection = t.child().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
              where: {parent}
              update:{{
                p: {{ set: "p2" }}
                childOpt: {{delete: true}}
              }}
              create:{{p: "Should not matter", p_1: "no", p_2: "yes"}}
            ){{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );
        Ok(())
    }

    // "a P1 to C1  relation" should "error if the nodes are not connected"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_error_if_not_connected(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{p: "p1", p_1: "p", p_2: "1"}}) {{
                        {selection}
                      }}
                    }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        assert_error!(
            runner,
            format!(r#"mutation {{
              upsertOneParent(
                where: {parent}
                update:{{
                  p: {{ set: "p2" }}
                  childOpt: {{delete: true}}
                }}
                create:{{p: "Should not matter", p_1: "nono", p_2: "yesyes"}}
              ){{
                childOpt {{
                  c
                }}
              }}
            }}"#, parent = parent),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested delete on relation 'ChildToParent'."
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_should_req(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childrenOpt: {{
                          create: {{c: "c1", c_1: "asdf", c_2: "qwer"}}
                        }}
                      }}){{
                        {selection}
                        childrenOpt{{
                          c
                        }}
                      }}
                    }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
              where: {parent}
              update:{{
                childrenOpt: {{delete: {{c: "c1"}}}}
              }}
              create:{{p: "Should not matter", p_1: "foo", p_2: "bar"}}
            ){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"upsertOneParent":{"childrenOpt":[]}}}"###
        );
        Ok(())
    }

    // "a P1 to C1!  relation " should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childOpt: {{
                          create: {{c: "c1", c_1: "foo", c_2: "bar"}}
                        }}
                      }}){{
                        {selection}
                        childOpt{{
                          c
                        }}
                      }}
                    }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
            where: {parent}
            update:{{
              childOpt: {{delete: true}}
            }}
            create:{{p: "Should not matter", p_1: "no", p_2: "yes"}}
            ){{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    // "a PM to C1 " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "foo", c_2: "bar"}}, {{c: "c2", c_1: "nono", c_2: "yesyes"}}]
                        }}
                      }}){{
                        {selection}
                        childrenOpt{{
                          c
                        }}
                      }}
                    }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
            where: {parent}
            update:{{
              childrenOpt: {{delete: [{{c: "c2"}}]}}
            }}
             create:{{p: "Should not matter", p_1: "no", p_2: "yes"}}
            ){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"upsertOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation" should "error"
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany")]
    async fn p1_req_cm_should_error(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childReq: {{
                          create: {{
                            c: "c1"
                            c_1: "c_1"
                            c_2: "c_2"
                          }}
                        }}
                      }}){{
                        {selection}
                        childReq{{
                          c
                        }}
                      }}
                    }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        assert_error!(
            runner,
            format!(r#"mutation {{
              upsertOneParent(
              where: {parent}
              update:{{
                childReq: {{delete: true}}
              }}
              create:{{p: "Should not matter", p_1: "nono", p_2: "noyes", childReq: {{create:{{c: "Should not matter", c_1: "foo", c_2: "bar"}}}}}}
              ){{
                childReq {{
                  c
                }}
              }}
            }}"#, parent = parent),
            2009,
            "`Mutation.upsertOneParent.update.ParentUpdateInput.childReq.ChildUpdateOneRequiredWithoutParentsOptInput.delete`: Field does not exist on enclosing type."
        );

        Ok(())
    }

    // "a P1 to CM  relation " should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1",
                        p_1: "p_1"
                        p_2: "p_2"
                        childOpt: {{
                          create: {{
                            c: "c1"
                            c_1: "c_1"
                            c_2: "c_2"
                          }}
                        }}
                      }}){{
                        {selection}
                        childOpt{{
                          c
                        }}
                      }}
                    }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
              where: {parent}
              update:{{childOpt: {{delete: true}}}}
              create:{{p: "Should not matter", p_1: "no", p_2: "yes"}}
            ){{
              childOpt{{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "foo", c_2: "bar"}},{{c: "c2", c_1: "wtf", c_2: "lol"}}]
                        }}
                      }}){{
                        {selection}
                        childrenOpt{{
                          c
                        }}
                      }}
                    }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
            where: {parent}
            update:{{
              childrenOpt: {{delete: [{{c: "c1"}}, {{c: "c2"}}]}}
            }}
            create:{{p: "Should not matter", p_1: "foo", p_2: "bar"}}
            ){{
              childrenOpt{{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"upsertOneParent":{"childrenOpt":[]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}
