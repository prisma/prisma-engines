use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
// update_many_inside_update
mod um_inside_update {
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams, Runner, TestResult};
    use query_test_macros::relation_link_test;

    // "A 1-n relation" should "error if trying to use nestedUpdateMany"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn one2n_rel_error_nested_um(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
                  childOpt: {{updateMany: {{
                      where:{{c: "c"}}
                      data: {{c: {{ set: "newC" }}}}

                  }}}}
              }}){{
                {selection}
                childOpt {{
                  c
                }}
              }}
            }}"#,
                parent = parent,
                selection = t.parent().selection()
            ),
            2009,
            "Field does not exist in enclosing type."
        );

        Ok(())
    }

    // "a PM to C1! relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                    childrenOpt: {{updateMany: {{
                        where: {{ c: {{ contains:"c"}} }}
                        data: {{ non_unique: {{ set: "updated" }}}}
                    }}}}
                  }}){{
                    childrenOpt {{
                      c
                      non_unique
                    }}
                  }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c, non_unique}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1","non_unique":"updated"},{"c":"c2","non_unique":"updated"}]},{"p":"p2","childrenOpt":[{"c":"c3","non_unique":null},{"c":"c4","non_unique":null}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                    childrenOpt: {{updateMany: {{
                        where: {{ c: {{ contains:"c" }} }}
                        data: {{ non_unique: {{ set: "updated" }}}}
                    }}}}
                  }}){{
                    childrenOpt {{
                      c
                      non_unique
                    }}
                  }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c, non_unique}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1","non_unique":"updated"},{"c":"c2","non_unique":"updated"}]},{"p":"p2","childrenOpt":[{"c":"c3","non_unique":null},{"c":"c4","non_unique":null}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                    childrenOpt: {{updateMany: {{
                        where: {{c: {{ contains:"c" }} }}
                        data: {{non_unique: {{ set: "updated" }}}}
                    }}}}
                  }}){{
                    childrenOpt {{
                      c
                      non_unique
                    }}
                  }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c, non_unique}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1","non_unique":"updated"},{"c":"c2","non_unique":"updated"}]},{"p":"p2","childrenOpt":[{"c":"c3","non_unique":null},{"c":"c4","non_unique":null}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "work with several updateManys"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_many_ums(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                    childrenOpt: {{updateMany: [
                    {{
                        where: {{c: {{ contains:"1" }} }}
                        data: {{non_unique: {{ set: "updated1" }}}}
                    }},
                    {{
                        where: {{c: {{ contains:"2" }} }}
                        data: {{non_unique: {{ set: "updated2" }}}}
                    }}
                    ]}}
                  }}){{
                    childrenOpt {{
                      c
                      non_unique
                    }}
                  }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c, non_unique}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1","non_unique":"updated1"},{"c":"c2","non_unique":"updated2"}]},{"p":"p2","childrenOpt":[{"c":"c3","non_unique":null},{"c":"c4","non_unique":null}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "work with empty Filter"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_empty_filter(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                    childrenOpt: {{updateMany: [
                    {{
                        where: {{}}
                        data: {{ non_unique: {{ set: "updated1" }}}}
                    }}
                    ]}}
                  }}){{
                    childrenOpt {{
                      c
                      non_unique
                    }}
                  }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c, non_unique}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1","non_unique":"updated1"},{"c":"c2","non_unique":"updated1"}]},{"p":"p2","childrenOpt":[{"c":"c3","non_unique":null},{"c":"c4","non_unique":null}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "not change anything when there is no hit"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_noop_no_hit(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                    childrenOpt: {{updateMany: [
                    {{
                        where: {{ c: {{ contains:"3" }}}}
                        data: {{ non_unique: {{ set: "updated3" }}}}
                    }},
                    {{
                        where: {{ c: {{ contains:"4" }}}}
                        data: {{ non_unique: {{ set: "updated4" }}}}
                    }}
                    ]}}
                  }}){{
                    childrenOpt {{
                      c
                      non_unique
                    }}
                  }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt{c, non_unique}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1","non_unique":null},{"c":"c2","non_unique":null}]},{"p":"p2","childrenOpt":[{"c":"c3","non_unique":null},{"c":"c4","non_unique":null}]}]}}"###
        );

        Ok(())
    }

    // optional ordering

    // "a PM to C1!  relation " should "work when multiple filters hit"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_many_filters(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = setup_data(runner, t).await?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                    childrenOpt: {{updateMany: [
                    {{
                        where: {{ c: {{ contains: "c" }}}}
                        data: {{ non_unique: {{ set: "updated1" }}}}
                    }},
                    {{
                        where: {{ c: {{ contains: "c1" }}}}
                        data: {{ non_unique: {{ set: "updated2" }}}}
                    }}
                    ]}}
                  }}){{
                    childrenOpt (orderBy: {{ c: asc }}){{
                      c
                      non_unique
                    }}
                  }}
                }}"#
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent{p,childrenOpt(orderBy: { c: asc }){c, non_unique}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1","non_unique":"updated2"},{"c":"c2","non_unique":"updated1"}]},{"p":"p2","childrenOpt":[{"c":"c3","non_unique":null},{"c":"c4","non_unique":null}]}]}}"###
        );

        Ok(())
    }

    async fn setup_data(runner: &Runner, t: &DatamodelWithParams) -> TestResult<String> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1"
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "foo", c_2: "bar"}},{{c: "c2", c_1: "fo", c_2: "lo"}}]
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

        run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p2", p_1: "p", p_2: "2"
                    childrenOpt: {{
                      create: [{{c: "c3", c_1: "ao", c_2: "bo"}},{{c: "c4", c_1: "go", c_2: "zo"}}]
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

        Ok(parent)
    }
}
