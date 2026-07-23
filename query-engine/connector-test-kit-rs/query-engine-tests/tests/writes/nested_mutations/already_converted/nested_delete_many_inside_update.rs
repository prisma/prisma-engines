use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod delete_many_inside_update {
    use query_engine_tests::{assert_error, run_query, run_query_json};
    use query_test_macros::relation_link_test;

    // "A 1-n relation" should "error if trying to use nestedDeleteMany"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn o2n_rel_fail(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
            format!(
                r#"mutation {{
              updateOneParent(
              where: {parent}
              data:{{
                p: {{ set: "p2" }}
                childOpt: {{ deleteMany: {{ where: {{ c: "c" }}}}}}
              }}) {{
                childOpt {{
                  c
                }}
              }}
            }}"#
            ),
            2009,
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let (parent_1, _) = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                    updateOneParent(
                      where: {parent_1}
                      data:{{
                      childrenOpt: {{deleteMany: {{c: {{ contains:"c"}} }}
                    }}
                    }}){{
                      childrenOpt {{
                        c

                      }}
                    }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let (parent_1, _) = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
          updateOneParent(
            where: {parent_1}
            data:{{
            childrenOpt: {{deleteMany: {{
                  c: {{ contains:"c" }}
              }}
            }}
          }}){{
            childrenOpt {{
              c

            }}
          }}
        }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "work with several deleteManys"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_many_delete_manys(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let (parent_1, _) = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                    updateOneParent(
                      where: {parent_1}
                      data:{{
                      childrenOpt: {{deleteMany: [
                        {{ c: {{ contains:"1" }} }},
                        {{ c: {{ contains:"2" }} }}
                      ]}}
                    }}) {{
                      childrenOpt {{
                        c
                      }}
                    }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1! relation " should "work with empty Filter"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_work_empty_filter(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let (parent_1, _) = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                    updateOneParent(
                      where: {parent_1}
                      data:{{
                      childrenOpt: {{ deleteMany: [{{}}] }}
                    }}){{
                      childrenOpt {{
                        c
                      }}
                    }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "not change anything when there is no hit"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_no_change_if_no_hit(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let (parent_1, _) = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                    updateOneParent(
                      where: {parent_1}
                      data:{{
                      childrenOpt: {{deleteMany: [
                        {{ c: {{ contains:"3" }} }},
                        {{ c: {{ contains:"4" }} }}
                      ]}}
                    }}){{
                      childrenOpt {{
                        c
                      }}
                    }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1"},{"c":"c2"}]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}"###
        );

        Ok(())
    }

    async fn setup_data(runner: &Runner, t: &DatamodelWithParams) -> TestResult<(String, String)> {
        let parent_1 = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: [{{c: "c1", c_1: "dear", c_2: "god"}},{{c: "c2", c_1: "why", c_2: "me"}}]
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

        let parent_2 = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p2", p_1: "p", p_2: "2"
                          childrenOpt: {{
                            create: [{{c: "c3", c_1: "buu", c_2: "huu"}},{{c: "c4", c_1: "meow", c_2: "miau"}}]
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

        Ok((parent_1, parent_2))
    }
}
