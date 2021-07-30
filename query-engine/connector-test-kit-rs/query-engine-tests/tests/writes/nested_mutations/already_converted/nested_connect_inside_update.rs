use query_engine_tests::*;

#[test_suite]
mod connect_inside_update {
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams};
    use query_test_macros::relation_link_test;

    // "a P1 to C1  relation with the child already in a relation" should "be connectable through a nested mutation if the child is already in a relation"
    // TODO(dom): Not working on mongo
    // panicked at 'not implemented: Compound filter case.', query-engine/connectors/mongodb-query-connector/src/filter.rs:107:13
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt", exclude(SqlServer, MongoDb))]
    async fn p1_c1_child_in_rel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let loose_child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "looseChild", c_1: "c", c_2: "1"}})
                        {{
                          {selection}
                        }}
                      }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
        )?;

        let other_parent_with_child = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data:{{
                          p: "otherParent", p_1: "p", p_2: "1",
                          childOpt: {{create: {{c: "otherChild", c_1: "c", c_2: "2"}}}}
                        }}){{
                          {selection}
                        }}
                      }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;

        let child_3 = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p2", p_1: "p", p_2: "2",
                            childOpt: {{
                                create: {{c: "c3", c_1: "c", c_2: "3"}}
                            }}
                        }}){{
                            childOpt{{
                                {selection}
                            }}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childOpt"],
        )?;

        let parent_3 = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p3", p_1: "p", p_2: "3",
                            childOpt: {{
                                create: {{c: "c4", c_1: "c", c_2: "4"}}
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
            updateOneParent(where: {parent_3}, data: {{ childOpt: {{ connect: {child_3} }} }}) {{
              childOpt {{
                c
              }}
            }}
          }}"#, parent_3 = parent_3, child_3 = child_3)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c3"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"query {{
            findUniqueParent(where: {other_parent_with_child}){{
              childOpt {{
                c
              }}
            }}
          }}"#, other_parent_with_child = other_parent_with_child)),
          @r###"{"data":{"findUniqueParent":{"childOpt":{"c":"otherChild"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"query {{
              findUniqueChild(where: {loose_child}){{
                c
              }}
            }}"#, loose_child = loose_child)),
          @r###"{"data":{"findUniqueChild":{"c":"looseChild"}}}"###
        );

        Ok(())
    }

    // "a P1 to C1 relation with the child and the parent without a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt", exclude(MongoDb))]
    async fn p1_c1_wo_parent_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().parse(
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

        let parent_1 = t.parent().parse(
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
            updateOneParent(where: {parent_1}, data: {{ childOpt: {{ connect: {child_1} }} }}) {{
              childOpt {{
                c
              }}
            }}
          }}"#, parent_1 = parent_1, child_1 = child_1)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1 relation with the child without a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt", exclude(SqlServer, MongoDb))]
    async fn p1_c1_child_wo_rel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
                        createOneParent(data: {{
                            p: "p1", p_1: "p", p_2: "1",
                            childOpt: {{
                                create: {{c: "c2", c_1: "c", c_2: "2"}}
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
            updateOneParent(where: {parent}, data: {{ childOpt: {{ connect: {child} }} }}) {{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation with the parent without a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt", exclude(SqlServer, MongoDb))]
    async fn p1_c1_parnt_wo_rel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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

        let child_id = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p2", p_1: "p", p_2: "2",
                            childOpt: {{
                                create: {{c: "c1", c_1: "c", c_2: "1"}}
                            }}
                        }}){{
                            childOpt{{
                                {selection}
                            }}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childOpt"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(where: {parent}, data: {{ childOpt: {{ connect: {child_id} }} }}) {{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child_id = child_id)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "A PM to CM relation connecting two nodes twice" should "not error"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_rel_connect_twice_error(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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

        let child = t.child().parse_many_first(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p2", p_1: "p", p_2: "2",
                            childrenOpt: {{
                                create: {{c: "c1", c_1: "c", c_2: "1"}}
                            }}
                        }}){{
                            childrenOpt{{
                                {selection}
                            }}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childrenOpt"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(where: {parent}, data: {{ childrenOpt: {{ connect: {child} }} }}) {{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(where: {parent}, data: {{ childrenOpt: {{ connect: {child} }} }}) {{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyParent {p, childrenOpt{ c }} }"#),
          @r###"{"data":{"findManyParent":[{"p":"p1","childrenOpt":[{"c":"c1"}]},{"p":"p2","childrenOpt":[{"c":"c1"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to C1! relation with the child already in a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq", exclude(MongoDb))]
    async fn pm_c1req_child_in_rel_connect(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let other_parent_with_child = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                createOneParent(data:{{
                  p: "otherParent", p_1: "p", p_2: "1",
                  childrenOpt: {{create: {{c: "otherChild", c_1: "c", c_2: "1"}}}}
                }}){{
                   {selection}
                }}
              }}"#,
                    selection = t.parent().selection()
                )
            ),
            &["data", "createOneParent"],
        )?;
        let child = t.child().parse_many_first(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                createOneParent(data: {{
                  p: "p2", p_1: "p", p_2: "2",
                  childrenOpt: {{
                    create: {{c: "c2", c_1: "c", c_2: "2"}}
                  }}
                }}){{
                  childrenOpt{{
                      {selection}
                  }}
                }}
              }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childrenOpt"],
        )?;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                    createOneParent(data: {{
                        p: "p3", p_1: "p", p_2: "3",
                        childrenOpt: {{
                            create: {{c: "c3", c_1: "c", c_2: "3"}}
                        }}
                    }}){{
                        childrenOpt{{
                            {selection}
                        }}
                    }}
                }}"#,
                selection = t.child().selection()
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {{p: "p3"}}
              data:{{
                childrenOpt: {{connect: {child}}}
              }}
            ){{
              childrenOpt(take:10, orderBy: {{ c: asc }}) {{
                c
              }}
            }}
          }}"#, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"},{"c":"c3"}]}}}"###
        );

        // verify preexisting data
        let res = run_query_json!(
            runner,
            format!(
                r#"query {{
                    findUniqueParent(where: {other_parent_with_child}){{
                        childrenOpt {{
                            c
                        }}
                    }}
                }}"#,
                other_parent_with_child = other_parent_with_child
            ),
            &["data", "findUniqueParent", "childrenOpt", "[0]", "c"]
        )
        .to_string();

        assert_eq!(res, "\"otherChild\"");

        Ok(())
    }

    // "a P1 to C1!  relation with the child and the parent already in a relation" should "should error in a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq", exclude(MongoDb))]
    async fn p1_c1req_rel_child_parnt_error(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p1", p_1: "p", p_2: "1",
                            childOpt: {{
                                create: {{c: "c1", c_1: "c", c_2: "1"}}
                            }}
                        }}){{
                            childOpt{{
                                {selection}
                            }}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childOpt"],
        )?;
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p2", p_1: "p", p_2: "2",
                            childOpt: {{
                                create: {{c: "c2", c_1: "c", c_2: "2"}}
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

        assert_error!(
            runner,
            format!(r#"mutation {{
                updateOneParent(
                where: {parent}
                data:{{
                  childOpt: {{connect: {child} }}
                }}){{
                  childOpt {{
                    c
                  }}
                }}
              }}"#, parent = parent, child = child),
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    // "a P1 to C1! relation with the child already in a relation" should "should not error when switching to a different parent"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq", exclude(MongoDb))]
    async fn p1_c1req_child_in_rel_no_error(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p1", p_1: "p", p_2: "1",
                            childOpt: {{
                                create: {{c: "c1", c_1: "c", c_2: "1"}}
                            }}
                        }}){{
                            childOpt{{
                                {selection}
                            }}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childOpt"],
        )?;
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p2", p_1: "p", p_2: "2",
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
              childOpt: {{connect: {child}}}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the child already in a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt", exclude(MongoDb))]
    async fn pm_c1_child_in_rel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneParent(data: {
                    p: "p1", p_1: "p", p_2: "1",
                    childrenOpt: {
                        create: [{c: "c1", c_1: "c", c_2: "1"}, {c: "c2", c_1: "c", c_2: "2"}, {c: "c3", c_1: "c", c_2: "3"}]
                    }
                }){
                    childrenOpt{
                        c
                    }
                }
            }"#
        );

        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{p: "p2", p_1: "p", p_2: "2"}}){{
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
              childrenOpt: {{connect: [{{c: "c1"}},{{c: "c2"}},{{c: "c2"}}]}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueParent(where:{p: "p1"}){childrenOpt{c}}}"#),
          @r###"{"data":{"findUniqueParent":{"childrenOpt":[{"c":"c3"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the child without a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt", exclude(MongoDb))]
    async fn pm_c1_child_wo_rel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
                        createOneParent(data: {{p: "p1", p_1: "p", p_2: "1",}}) {{
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
              childrenOpt: {{connect: {child}}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation with the child already in a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany", exclude(MongoDb))]
    async fn p1req_cm_child_inrel_connect(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p1", p_1: "p", p_2: "1",
                            childReq: {{
                                create: {{c: "c1", c_1: "c", c_2: "1"}}
                            }}
                        }}){{
                            childReq{{
                                {selection}
                            }}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childReq"],
        )?;
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p2", p_1: "p", p_2: "2",
                            childReq: {{
                                create: {{c: "c2", c_1: "c", c_2: "2"}}
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
              childReq: {{connect: {child}}}
            }}){{
              childReq {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childReq":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c2","parentsOpt":[]}]}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation with the child not already in a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany", exclude(MongoDb))]
    async fn p1req_cm_child_norel_connect(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "c1", c_1: "c", c_2: "1"}}){{
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
                        createOneParent(data: {{
                            p: "p1", p_1: "p", p_2: "1",
                            childReq: {{
                                create: {{c: "c2", c_1: "c", c_2: "2"}}
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
              childReq: {{connect: {child}}}
            }}){{
              childReq {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childReq":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[]}]}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation with the child already in a relation" should "be connectable through a nested mutation"
    // TODO:(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany", exclude(MongoDb))]
    async fn p1_cm_child_in_rel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1",
                          childOpt: {{
                            create: {{c: "c1", c_1: "c", c_2: "1"}}
                          }}
                        }}){{
                          childOpt{{
                             {selection}
                          }}
                        }}
                      }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childOpt"],
        )?;
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{p: "p2", p_1: "p", p_2: "2"}}){{
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
              childOpt: {{connect: {child}}}
            }}){{
              childOpt{{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation with the child not already in a relation" should "be connectable through a nested mutation"
    // TODO(dom): Not working on mongo
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany", exclude(MongoDb))]
    async fn p1_cm_child_norel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "c1", c_1: "c", c_2: "1"}}){{
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
              childOpt: {{connect: {child}}}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be connectable through a nested mutation"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let children = t.child().parse_many_all(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p1", p_1: "p", p_2: "1",
                            childrenOpt: {{
                                create: [{{c: "c1", c_1: "c", c_2: "1"}},{{c: "c2", c_1: "c", c_2: "2"}}]
                            }}
                        }}){{
                            childrenOpt{{
                                {selection}
                            }}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneParent", "childrenOpt"],
        )?;
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p2", p_1: "p", p_2: "2",
                          childrenOpt: {{
                            create: [{{c: "c3", c_1: "c", c_2: "3"}},{{c: "c4", c_1: "c", c_2: "4"}}]
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
              childrenOpt: {{connect: {children}}}
            }}){{
              childrenOpt(orderBy: {{ c: asc }}){{
                c
              }}
            }}
          }}"#, parent = parent, children = children)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c3"},{"c":"c4"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c2","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c3","parentsOpt":[{"p":"p2"}]},{"c":"c4","parentsOpt":[{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the child not already in a relation" should "be connectable through a nested mutation"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_norel_connect_mut(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "c1", c_1: "c", c_2: "1"}}){{
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
              childrenOpt: {{connect: {child}}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }
}
