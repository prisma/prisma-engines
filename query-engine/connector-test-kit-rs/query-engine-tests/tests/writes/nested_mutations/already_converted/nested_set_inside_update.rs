use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod set_inside_update {
    use query_engine_tests::{DatamodelWithParams, run_query, run_query_json};
    use query_test_macros::relation_link_test;

    // "a PM to C1  relation with the child already in a relation" should "be setable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_child_inrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [{{c: "c1", c_1: "c1", c_2: "c2"}}, {{c: "c2", c_1: "c3", c_2: "c4"}}]
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
        );
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{p: "p2", p_1: "wqe", p_2: "qt12t"}}){{
                        p
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
              childrenOpt: {{set: [{{c: "c1"}},{{c: "c2"}},{{c: "c2"}}]}}
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

    // "a PM to C1  relation with the child already in a relation" should "be setable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_child_inrel_with_filters(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [{{c: "c1", c_1: "c1", c_2: "c2", non_unique: "0"}}, {{c: "c2", c_1: "c3", c_2: "c4", non_unique: "1" }}]
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
        );
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{p: "p2", p_1: "wqe", p_2: "qt12t"}}){{
                        p
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
              childrenOpt: {{set: [{{c: "c1", non_unique: "0"}},{{c: "c2", non_unique: "1" }},{{c: "c2", non_unique: "1"}}]}}
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

    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_child_inrel_fails_if_no_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [{{c: "c1", c_1: "c1", c_2: "c2", non_unique: "0"}}, {{c: "c2", c_1: "c3", c_2: "c4", non_unique: "1" }}]
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
        );
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{p: "p2", p_1: "wqe", p_2: "qt12t"}}){{
                        p
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
              childrenOpt: {{set: [{{c: "c1", non_unique: "1"}}]}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[]}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the child without a relation" should "be setable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_child_wo_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneChild(data: {{c: "c1", c_1: "c", c_2: "1"}}) {{
                        {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
        )?;
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
              childrenOpt: {{set: {child}}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be setable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [
                        {{c: "c1", c_1: "c", c_2: "1"}},
                        {{c: "c2", c_1: "c", c_2: "2"}}
                      ]
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
        );

        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p2", p_1: "p", p_2: "2"
                        childrenOpt: {{
                          create: [
                            {{c: "c3", c_1: "c", c_2: "3"}},
                            {{c: "c4", c_1: "c", c_2: "4"}}
                          ]
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
                childrenOpt: {{set: [{{c: "c1"}}, {{c: "c2"}}]}}
            }}){{
              childrenOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c2","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c3","parentsOpt":[]},{"c":"c4","parentsOpt":[]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the child not already in a relation" should "be setable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_notinrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneChild(data: {{c: "c1", c_1: "c", c_2: "1"}}){{
                          c
                          {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
        )?;
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{p: "p1", p_1: "p", p_2: "1"}}){{
                          p
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
              childrenOpt: {{set: {child}}}
          }}){{
            childrenOpt {{
              c
            }}
          }}
        }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be setable to empty"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel_set_empty(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [
                        {{c: "c1", c_1: "c", c_2: "1"}},
                        {{c: "c2", c_1: "c", c_2: "2"}}
                      ]
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
        );
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p2", p_1: "p", p_2: "2"
                        childrenOpt: {{
                          create: [{{c: "c3", c_1: "u", c_2: "w"}},{{c: "c4", c_1: "g", c_2: "l"}}]
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
                childrenOpt: {{set: []}}
            }}){{
              childrenOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild(orderBy: { c: asc }){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c3","parentsOpt":[]},{"c":"c4","parentsOpt":[]}]}}"###
        );

        Ok(())
    }
}
