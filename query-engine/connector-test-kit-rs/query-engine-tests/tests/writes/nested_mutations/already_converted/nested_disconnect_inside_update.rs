use query_engine_tests::*;

#[test_suite]
mod disconnect_inside_update {
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams};
    use query_test_macros::relation_link_test;

    // "a P1 to C1 relation " should "be disconnectable through a nested mutation by id"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
              childOpt: {{disconnect: true}}
          }}){{
            childOpt {{
              c
            }}
          }}
        }}"#, parent = parent)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    // "a P1 to C1 relation with the child and the parent without a relation" should "be disconnectable through a nested mutation by id"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_child_wo_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            format!(
                r#"mutation {{
          createOneChild(data: {{
            c: "c1"
            c_1: "c_1"
            c_2: "c_2"
          }}) {{
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
            updateOneParent(
            where: {parent}
            data:{{
              p: {{ set: "p2" }}
              childOpt: {{ disconnect: true }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation with the child already in a relation" should "not be disconnectable through a nested mutation by unique"
    // TODO(dom & flavian): Am I dumb? What's the error message diff here?
    // #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    // async fn pm_c1_rel_child_inrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
    //     let parent_result = run_query_json!(
    //         runner,
    //         format!(
    //             r#"mutation {{
    //       createOneParent(data: {{
    //         p: "p1", p_1: "p", p_2: "1"
    //         childrenOpt: {{
    //           create: {{ c: "c1", c_1: "c", c_2: "1" }}
    //         }}
    //       }}){{
    //         {parent_selection}
    //         childrenOpt{{
    //            {child_selection}
    //         }}
    //       }}
    //     }}"#,
    //             parent_selection = t.parent().selection(),
    //             child_selection = t.child().selection()
    //         )
    //     );
    //     let parent = t.parent().parse(parent_result.clone(), &["data", "createOneParent"])?;
    //     let child_ids = t
    //         .child()
    //         .parse_many(parent_result, &["data", "createOneParent", "childrenOpt"])?;
    //     let child = child_ids.first().unwrap();

    //     assert_error!(
    //         runner,
    //         format!(r#"mutation {{
    //           updateOneParent(
    //             where: {parent}
    //             data:{{
    //               childrenOpt: {{disconnect: {child} }}
    //           }}){{
    //             childrenOpt {{
    //               c
    //             }}
    //           }}
    //         }}"#, parent = parent, child = child),
    //         2014,
    //         "Error in query graph construction: RelationViolation(RelationViolation { relation_name: \\\"ChildToParent\\\", model_a_name: \\\"Child\\\", model_b_name: \\\"Parent\\\""
    //     );
    //     Ok(())
    // }

    // "a P1 to C1!  relation with the child and the parent already in a relation" should "should error in a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_child_par_inrel_error(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
              createOneParent(data: {{
                p: "p1", p_1: "p", p_2: "1"
                childOpt: {{
                  create: {{ c: "c1", c_1: "c", c_2: "1" }}
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

        assert_error!(
            runner,
            format!(r#"mutation {{
              updateOneParent(
              where: {parent}
              data:{{
                childOpt: {{disconnect: true}}
              }}){{
                childOpt {{
                  c
                }}
              }}
            }}"#, parent = parent),
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    // "a PM to C1 relation with the child already in a relation" should "be disconnectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_child_inrel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [
                        {{ c: "c1", c_1: "c", c_2: "1" }},
                        {{ c: "c2", c_1: "c", c_2: "2" }}
                      ]
                    }}
                  }}){{
                    {parent_selection}
                    childrenOpt{{
                       {child_selection}
                    }}
                  }}
                }}"#,
                parent_selection = t.parent().selection(),
                child_selection = t.child().selection()
            )
        );
        let parent = t.parent().parse(res.clone(), &["data", "createOneParent"])?;
        let second_child_ids = t.child().parse_many(res, &["data", "createOneParent", "childrenOpt"])?;
        let second_child = second_child_ids.get(1).unwrap();

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
            where: {parent}
            data:{{
              childrenOpt: {{disconnect: [{child}]}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = second_child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
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
            updateOneParent(
              where: {parent}
              data:{{
              childOpt: {{disconnect: true}}
            }}){{
              childOpt{{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
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
        // Note for review
        // we were relying of the order of the returned child ids without specifying an order by.
        // with the direct return of the manyrecord that order seems to have changed in the case where we return the id field
        // that means depending on whether you have queryargs that do nothing or not your order might change -.-
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [
                        {{ c: "c1", c_1: "c", c_2: "1" }},
                        {{ c: "c2", c_1: "c", c_2: "2" }}
                      ]
                    }}
                  }}){{
                    {parent_selection}
                    childrenOpt(orderBy: {{ id: asc }}){{
                       {child_selection}
                    }}
                  }}
                }}"#,
                parent_selection = t.parent().selection(),
                child_selection = t.child().selection()
            )
        );
        let parent = t.parent().parse(res.clone(), &["data", "createOneParent"])?;
        let first_child_ids = t.child().parse_many(res, &["data", "createOneParent", "childrenOpt"])?;
        let first_child = first_child_ids.first().unwrap();

        let other_parent_res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "otherParent", p_1: "otherParent_1", p_2: "otherParent_2"
                    childrenOpt: {{
                      create: [
                        {{ c: "otherChild", c_1: "otherChild_1", c_2: "otherChild_2" }}
                      ]
                      connect: [{first_child}]
                    }}
                  }}){{
                    childrenOpt(orderBy: {{ id: asc }}){{
                      {selection}
                    }}
                  }}
                }}"#,
                first_child = first_child,
                selection = t.child().selection()
            )
        );
        let other_child_ids = t
            .child()
            .parse_many(other_parent_res, &["data", "createOneParent", "childrenOpt"])?;
        let other_child = other_child_ids.get(1).unwrap();

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
            where: {parent}
            data:{{
              childrenOpt: {{disconnect: []}}
            }}){{
              childrenOpt{{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                      childrenOpt: {{disconnect: [{first_child}, {other_child}]}}
                  }}){{
                    childrenOpt{{
                      c
                    }}
                  }}
                }}"#,
                parent = parent,
                first_child = first_child,
                other_child = other_child
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"c1"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"c1","parentsOpt":[{"p":"otherParent"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"c2"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"c2","parentsOpt":[{"p":"p1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"otherChild"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"otherChild","parentsOpt":[{"p":"otherParent"}]}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be disconnectable through a nested mutation by unique 2"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel_2(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{ createOneParent(data: {{
                  p: "p1", p_1: "p", p_2: "1"
                  childrenOpt: {{
                    create: [
                      {{ c: "c1", c_1: "c", c_2: "1" }},
                      {{ c: "c2", c_1: "c", c_2: "2" }}
                    ]
                  }}
                }}){{
                  {parent_selection}
                  childrenOpt(orderBy: {{ id: asc }}){{
                     {child_selection}
                  }}
                }}
              }}"#,
                parent_selection = t.parent().selection(),
                child_selection = t.child().selection()
            )
        );
        let parent = t.parent().parse(res.clone(), &["data", "createOneParent"])?;
        let child_1_ids = t.child().parse_many(res, &["data", "createOneParent", "childrenOpt"])?;
        let child_1 = child_1_ids.first().unwrap();

        let other_parent_res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
              createOneParent(data: {{
                p: "otherParent", p_1: "otherParent_1", p_2: "otherParent_2"
                childrenOpt: {{
                  create: [
                    {{ c: "otherChild", c_1: "otherChild_1", c_2: "otherChild_2" }}
                  ]
                  connect: [{child_1}]
                }}
              }}){{
                childrenOpt(orderBy: {{ id: asc }}){{
                   {child_selection}
                }}
              }}
            }}"#,
                child_1 = child_1,
                child_selection = t.child().selection()
            )
        );
        let other_child_ids = t
            .child()
            .parse_many(other_parent_res, &["data", "createOneParent", "childrenOpt"])?;
        let other_child = other_child_ids.get(1).unwrap();

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                      childrenOpt: {{disconnect: [{child_1}, {child_2}]}}
                  }}){{
                    childrenOpt{{
                      c
                    }}
                  }}
                }}"#,
                parent = parent,
                child_1 = child_1,
                child_2 = other_child
            )
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be disconnectable through a nested mutation by unique 3"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel_3(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{ createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1"
                    childrenOpt: {{
                      create: [
                        {{ c: "c1", c_1: "c", c_2: "1" }},
                        {{ c: "c2", c_1: "c", c_2: "2" }}
                      ]
                    }}
                  }}){{
                    {parent_selection}
                    childrenOpt{{
                      {child_selection}
                    }}
                  }}
                }}"#,
                parent_selection = t.parent().selection(),
                child_selection = t.child().selection()
            )
        );
        let parent = t.parent().parse(res.clone(), &["data", "createOneParent"])?;
        let child_1_ids = t.child().parse_many(res, &["data", "createOneParent", "childrenOpt"])?;
        let child_1 = child_1_ids.first().unwrap();

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "otherParent", p_1: "otherParent_1", p_2: "otherParent_2"
                    childrenOpt: {{
                      create: [{{ c: "otherChild", c_1: "otherChild_1", c_2: "otherChild_2" }}]
                      connect: [{child_1}]
                    }}
                  }}){{
                    childrenOpt{{
                      {selection}
                    }}
                  }}
                }}"#,
                child_1 = child_1,
                selection = t.child().selection()
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{disconnect: [{child}]}}
            }}){{
              childrenOpt{{
                c
              }}
            }}
          }}"#, parent = parent, child = child_1)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"c1"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"c1","parentsOpt":[{"p":"otherParent"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"c2"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"c2","parentsOpt":[{"p":"p1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"otherChild"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"otherChild","parentsOpt":[{"p":"otherParent"}]}}}"###
        );

        Ok(())
    }
}
