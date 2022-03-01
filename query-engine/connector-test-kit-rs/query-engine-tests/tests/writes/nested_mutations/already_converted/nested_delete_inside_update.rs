use query_engine_tests::*;

#[test_suite]
mod delete_inside_update {
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams};
    use query_test_macros::relation_link_test;

    // "a P1 to C1  relation " should "work through a nested mutation by id"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_mut_by_id(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "existingParent", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "existingChild", c_1: "c", c_2: "1"}}
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
        let parent_2 = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p2", p_1: "p", p_2: "2",
                        childOpt: {{
                          create: {{c: "c2",, c_1: "c", c_2: "2"}}
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
              childOpt: {{delete: true}}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent_2)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        // Verify existing data

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"{{
            findUniqueParent(where: {parent} ){{
              childOpt {{
                c
              }}
            }}
          }}"#, parent = parent)),
          @r###"{"data":{"findUniqueParent":{"childOpt":{"c":"existingChild"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation" should "error if the nodes are not connected"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_error_if_not_connected(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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

        assert_error!(
            runner,
            format!(r#"mutation {{
              updateOneParent(
              where: {parent}
              data:{{
                childOpt: {{delete: true}}
              }}){{
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
    async fn pm_c1_req_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
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
                        {parent_selection}
                        childrenOpt {{
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
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"query {{
                      findUniqueChild(where: {{c: "c1"}}){{
                        {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "findUniqueChild"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{delete: {child}}}
              }}
            ){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
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
                        p: "p1", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "c1", c_1: "c", c_2: "1"}}
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
              childOpt: {{delete: true}}
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

    // "a PM to C1 " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "c", c_2: "1"}}, {{c: "c2", c_1: "c", c_2: "2"}}]
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
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"query {{
                      findUniqueChild(where: {{c: "c1"}}){{
                        {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "findUniqueChild"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
            where: {parent}
            data:{{
              childrenOpt: {{delete: [{child}]}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
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
                        p: "p1", p_1: "p", p_2: "1",
                        childReq: {{
                          create: {{c: "c1", c_1: "c", c_2: "1"}}
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
                childReq: {{delete: true}}
              }}){{
                childReq {{
                  c
                }}
              }}
            }}"#, parent = parent),
            2009,
            "`Mutation.updateOneParent.data.ParentUpdateInput.childReq.ChildUpdateOneRequiredWithoutParentsOptInput.delete`: Field does not exist on enclosing type."
        );
        Ok(())
    }

    // "a P1 to CM  relation " should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany", exclude(SqlServer))]
    async fn p1_cm_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
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
              childOpt: {{delete: true}}
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
          @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().parse_many_first(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "otherParent", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "otherChild", c_1: "c", c_2: "1"}}]
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
            run_query_json!(runner, format!(r#"mutation {{
              createOneParent(data: {{
                p: "p2", p_1: "p", p_2: "2",
                childrenOpt: {{
                  create: [{{c: "c2", c_1: "c", c_2: "2"}},{{c: "c3", c_1: "c", c_2: "3"}},{{c: "c4", c_1: "c", c_2: "4"}}]
                }}
              }}){{
                {selection}
              }}
            }}"#, selection = t.parent().selection())),
            &["data", "createOneParent"]
        )?;
        let child_2 = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"query {{
                      findUniqueChild(where: {{c: "c2"}}){{
                        {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "findUniqueChild"],
        )?;

        assert_error!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                      childrenOpt: {{delete: [{child_1}, {child_2}]}}
                  }}){{
                    childrenOpt{{
                      c
                    }}
                  }}
                }}"#,
                parent = parent,
                child_1 = child_1,
                child_2 = child_2
            ),
            2017,
            "The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected."
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{delete: [{child_2}]}}
            }}){{
              childrenOpt{{
                c
              }}
            }}
          }}"#, parent = parent, child_2 = child_2)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c3"},{"c":"c4"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"c4"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"c4","parentsOpt":[{"p":"p2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"otherChild"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"otherChild","parentsOpt":[{"p":"otherParent"}]}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation" should "error on invalid child"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_error_invalid_child(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().parse_many_first(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "otherParent", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "otherChild", c_1: "c", c_2: "1"}}]
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
            run_query_json!(runner, format!(r#"mutation {{
              createOneParent(data: {{
                p: "p2", p_1: "p", p_2: "2",
                childrenOpt: {{
                  create: [{{c: "c2", c_1: "c", c_2: "2"}},{{c: "c3", c_1: "c", c_2: "3"}},{{c: "c4", c_1: "c", c_2: "4"}}]
                }}
              }}){{
                {selection}
              }}
            }}"#, selection = t.parent().selection())),
            &["data", "createOneParent"]
        )?;
        let child_2 = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"query {{
                      findUniqueChild(where: {{c: "c2"}}){{
                        {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "findUniqueChild"],
        )?;

        assert_error!(
            runner,
            format!(
                r#"mutation {{
                  updateOneParent(
                    where: {parent}
                    data:{{
                      childrenOpt: {{delete: [{child_1}, {child_2}]}}
                  }}){{
                    childrenOpt{{
                      c
                    }}
                  }}
                }}"#,
                parent = parent,
                child_1 = child_1,
                child_2 = child_2
            ),
            2017,
            "The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected"
        );

        Ok(())
    }

    // "a PM to CM  relation" should "work for correct children"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_work_correct_children(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneParent(data: {
                  p: "otherParent", p_1: "p", p_2: "1",
                  childrenOpt: {
                    create: [{c: "otherChild", c_1: "c", c_2: "1"}]
                  }
                }){
                  childrenOpt{
                    c
                  }
                }
            }"#
        );
        let parent = t.parent().parse(
            run_query_json!(runner, format!(r#"mutation {{
              createOneParent(data: {{
                p: "p2", p_1: "p", p_2: "2",
                childrenOpt: {{
                  create: [{{c: "c2", c_1: "c", c_2: "2"}},{{c: "c3", c_1: "c", c_2: "3"}},{{c: "c4", c_1: "c", c_2: "4"}}]
                }}
              }}){{
                {selection}
              }}
            }}"#, selection = t.parent().selection())),
            &["data", "createOneParent"]
        )?;
        let child_2 = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"query {{
                      findUniqueChild(where: {{c: "c2"}}){{
                        {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "findUniqueChild"],
        )?;
        let child_3 = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"query {{
                      findUniqueChild(where: {{c: "c3"}}){{
                        {selection}
                      }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "findUniqueChild"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
          updateOneParent(
            where: {parent}
            data:{{
              childrenOpt: {{delete: [{child_2}, {child_3}]}}
          }}){{
            childrenOpt{{
              c
            }}
          }}
        }}"#, parent = parent, child_2 = child_2, child_3 = child_3)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c4"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyParent {p, childrenOpt{c}}}"#),
          @r###"{"data":{"findManyParent":[{"p":"otherParent","childrenOpt":[{"c":"otherChild"}]},{"p":"p2","childrenOpt":[{"c":"c4"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"c4"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"c4","parentsOpt":[{"p":"p2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findUniqueChild(where:{c:"otherChild"}){c, parentsOpt{p}}}"#),
          @r###"{"data":{"findUniqueChild":{"c":"otherChild","parentsOpt":[{"p":"otherParent"}]}}}"###
        );

        Ok(())
    }
}
