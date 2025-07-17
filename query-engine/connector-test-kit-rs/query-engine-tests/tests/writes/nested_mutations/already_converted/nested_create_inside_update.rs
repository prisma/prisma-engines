use query_engine_tests::*;

//TODO: which tests to keep and which ones to delete???? Some do not really test the compound unique functionality
// TODO(dom): All failing except one
#[test_suite(exclude(CockroachDb))]
mod create_inside_update {
    use query_engine_tests::{DatamodelWithParams, assert_error, run_query, run_query_json};
    use query_test_macros::relation_link_test;

    // "a P1! to C1 relation" should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
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
                    }}
                }}"#,
                selection = t.parent().selection()
            )
        );
        let parent_id = t.parent().parse(res, &["data", "createOneParent"])?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent_id}
              data:{{
                p: {{ set: "p2" }}
                childOpt: {{create: {{
                  c: "SomeC"
                  c_1: "c_1_1"
                  c_2: "c_2_2"
                }}}}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"SomeC"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation with the parent without a relation"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_parent_wo_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
            where: {parent}
            data:{{
              p: {{ set: "p2" }}
              childOpt: {{create: {{c: "SomeC", c_1: "Some1", c_2: "Some2"}}}}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"SomeC"}}}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation with a child already in a relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_child_in_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: {{c: "c1", c_1: "c_1", c_2: "c_2"}}
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
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{create: {{c: "c2", c_1: "foo", c_2: "bar"}}}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // "a P1 to C1!  relation with the parent and a child already in a relation" should "error in a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_par_child_in_rel_fail(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childOpt: {{
                            create: {{c: "c1", c_1: "1", c_2: "2"}}
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
            format!(
                r#"mutation {{
              updateOneParent(
              where: {parent}
              data:{{
                childOpt: {{create: {{c: "c2", c_1: "foo", c_2: "bar"}}}}
              }}){{
                childOpt {{
                  c
                }}
              }}
            }}"#
            ),
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    // "a P1 to C1!  relation with the parent not already in a relation" should "work in a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_par_not_in_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
              createOneParent(data: {{
                p: "p1", p_1: "p", p_2: "1"
              }}){{
                {selection}
                p
              }}
            }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
            where: {parent}
            data:{{
              childOpt: {{create: {{
                c: "c1"
                c_1: "c_1"
                c_2: "c_2"
              }}}}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the parent already in a relation" should "work through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_parent_in_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: [{{c: "c1", c_1: "foo", c_2: "bar"}}, {{c: "c2", c_1: "foobs", c_2: "lulz"}}]
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
            updateOneParent(
            where: {parent}
            data:{{
              childrenOpt: {{create: [{{c: "c3", c_1: "crazy", c_2: "I'm going"}}]}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c3"}]}}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation with the parent already in a relation" should "work through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany")]
    async fn p1_req_cm_parent_in_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
            where: {parent}
            data:{{
              childReq: {{create: {{
                c: "c2"
                c_1: "c_1_2"
                c_2: "c_2_2"
              }}}}
            }}){{
              childReq {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childReq":{"c":"c2"}}}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation with the child already in a relation" should "work through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm_child_in_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
            updateOneParent(
              where: {parent}
              data:{{
              childOpt: {{create: {{
                c: "c2"
                c_1: "c_1_2"
                c_2: "c_2_2"
              }}}}
            }}){{
              childOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c2"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[]},{"c":"c2","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be disconnectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_in_rel_disconnect(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: [{{c: "c1", c_1: "foo", c_2: "bar"}},{{c: "c2", c_1: "how many", c_2: "ids left"}}]
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
            updateOneParent(
            where: {parent}
            data:{{
              childrenOpt: {{create: [{{c: "c3", c_1: "no for christ", c_2: "sake"}}]}}
            }}){{
              childrenOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c3"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c3","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }
}
