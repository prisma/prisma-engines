use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod disconnect_inside_upsert {
    use query_engine_tests::{assert_error, run_query, run_query_json};
    use query_test_macros::relation_link_test;

    // "a P1 to C1 relation " should "be disconnectable through a nested mutation by id"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_by_id_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1"
                    p_1: "p"
                    p_2: "1"
                    childOpt: {{
                      create: {{
                        c: "c1"
                        c_1: "c_1"
                        c_2: "c_2"
                      }}
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
        );
        let parent = t.parent().parse(res, &["data", "createOneParent"])?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
              where: {parent}
              update:{{
                p: {{ set: "p2" }}
                childOpt: {{disconnect: true}}
              }}
              create:{{p: "Should not Matter", p_1: "lol", p_2: "woot"}}
            ){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    // "a P1 to C1 relation " should "be disconnectable through a nested mutation by id"
    #[relation_link_test(
        on_parent = "ToOneOpt",
        on_child = "ToOneOpt",
        capabilities(FilteredInlineChildNestedToOneDisconnect)
    )]
    async fn p1_c1_by_filters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1"
                    p_1: "p"
                    p_2: "1"
                    childOpt: {{
                      create: {{
                        c: "c1"
                        c_1: "c_1"
                        c_2: "c_2",
                        non_unique: "0"
                      }}
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
        );
        let parent = t.parent().parse(res, &["data", "createOneParent"])?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
              where: {parent}
              update:{{
                p: {{ set: "p2" }}
                childOpt: {{disconnect: {{ non_unique: "0" }} }}
              }}
              create:{{p: "Should not Matter", p_1: "lol", p_2: "woot"}}
            ){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    // "a P1 to C1 relation " should "be disconnectable through a nested mutation by id"
    // TODO: MongoDB doesn't support joins on top-level updates. It should be un-excluded once we fix that.
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt", exclude(MongoDb))]
    async fn p1_c1_by_fails_if_filter_no_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1"
                    p_1: "p"
                    p_2: "1"
                    childOpt: {{
                      create: {{
                        c: "c1"
                        c_1: "c_1"
                        c_2: "c_2",
                        non_unique: "0"
                      }}
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
        );
        let parent = t.parent().parse(res, &["data", "createOneParent"])?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
              where: {parent}
              update:{{
                p: {{ set: "p2" }}
                childOpt: {{disconnect: {{ non_unique: "1" }} }}
              }}
              create:{{p: "Should not Matter", p_1: "lol", p_2: "woot"}}
            ){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"upsertOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1 relation with the child and the parent without a relation" should "be disconnectable through a nested mutation by id"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_child_parnt_wo_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneChild(data: {{c: "c1", c_1: "c", c_2: "1"}}) {{
                    {selection}
                  }}
                }}"#,
                selection = t.child().selection()
            )
        );
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

        // Disconnect is a noop
        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            upsertOneParent(
              where: {parent}
              update:{{
                p: {{ set: "p2" }}
                childOpt: {{disconnect: true}}
              }}
              create: {{
                p:"Should not Matter"
                p_1:"lol"
                p_2:"woot"
              }}
            ){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );
        Ok(())
    }

    // "a PM to C1!  relation with the child already in a relation" should "not be disconnectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_child_inrel_noop(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childrenOpt: {{
                          create: {{c: "c1", c_1: "foo", c_2: "bar"}}
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

        assert_error!(
            runner,
            format!(r#"mutation {{
              upsertOneParent(
                where: {parent}
                update:{{
                childrenOpt: {{disconnect: {{c: "c1"}}}}
                }}
                create: {{p: "Should not Matter", p_1: "asd", p_2: "asdaf"}}
              ){{
                childrenOpt {{
                  c
                }}
              }}
            }}"#),
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );
        Ok(())
    }

    // "a P1 to C1!  relation with the child and the parent already in a relation" should "should error in a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_child_parnt_inrel_error(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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

        assert_error!(
            runner,
            format!(r#"mutation {{
              upsertOneParent(
              where: {parent}
              update:{{
                childOpt: {{disconnect: true}}
              }}
              create: {{p: "Should not Matter", p_1: "foo", p_2: "bar"}}
              ){{
                childOpt {{
                  c
                }}
              }}
            }}"#),
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );
        Ok(())
    }

    // "a PM to C1  relation with the child already in a relation" should "be disconnectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_child_inrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
              createOneParent(data: {{
                p: "p1", p_1: "p", p_2: "1"
                childrenOpt: {{
                  create: [{{c: "c1", c_1: "foo", c_2: "bar"}}, {{c: "c2", c_1: "qaw1", c_2: "qqw3"}}]
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
              childrenOpt: {{disconnect: [{{c: "c2"}}]}}
            }}
            create: {{p: "Should not Matter", p_1: "foo", p_2: "bar"}}
            ){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"upsertOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation with the child already in a relation" should "be disconnectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm_child_inrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
              createOneParent(data: {{
                p: "p1", p_1: "p", p_2: "1"
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
              update:{{
              childOpt: {{disconnect: true}}
            }}
              create: {{p: "Should not Matter", p_1: "foo", p_2: "bar"}}
            ){{
              childOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be disconnectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
              createOneParent(data: {{
                p: "p1", p_1: "p", p_2: "1"
                childrenOpt: {{
                  create: [{{c: "c1", c_1: "foo", c_2: "bar"}},{{c: "c2", c_1: "q124", c_2: "qawe"}}]
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
              childrenOpt: {{disconnect: [{{c: "c1"}}]}}
            }}
            create: {{p: "Should not Matter", p_1: "foo", p_2: "bar"}}
            ){{
              childrenOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"upsertOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[]},{"c":"c2","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }
}
