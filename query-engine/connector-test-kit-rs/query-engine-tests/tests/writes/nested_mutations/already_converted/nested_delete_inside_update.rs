use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod delete_inside_update {
    use query_engine_tests::{DatamodelWithParams, assert_error, run_query, run_query_json};
    use query_test_macros::relation_link_test;

    // ----------------------------------
    // ------------ P1 to C1 ------------
    // ----------------------------------

    // "a P1 to C1  relation "should "work through a nested mutation by id"
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
            where: {parent_2}
            data:{{
              childOpt: {{delete: true}}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
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
          }}"#)),
          @r###"{"data":{"findUniqueParent":{"childOpt":{"c":"existingChild"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation "should "work through a nested mutation by id & additional filters"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_mut_by_id_and_filters(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "existingParent", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "existingChild", c_1: "c", c_2: "1", non_unique: "0"}}
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
                          create: {{c: "c2", c_1: "c", c_2: "2", non_unique: "0"}}
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

        // Delete parent2
        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
            where: {parent_2}
            data:{{
              childOpt: {{ delete: {{ non_unique: "0" }} }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        // Verify existing data (parent1 should not be deleted)
        insta::assert_snapshot!(
          run_query!(runner, format!(r#"{{
            findUniqueParent(where: {parent} ){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"findUniqueParent":{"childOpt":{"c":"existingChild"}}}}"###
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
            format!(
                r#"mutation {{
              updateOneParent(
              where: {parent}
              data:{{
                childOpt: {{delete: true}}
              }}){{
                childOpt {{
                  c
                }}
              }}
            }}"#
            ),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested delete on one-to-one relation 'ChildToParent'."
        );

        Ok(())
    }

    // "a P1 to C1  relation" should "error if the node is connected but the additional filters don't match it"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_error_if_filter_not_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "existingChild", c_1: "c", c_2: "1", non_unique: "0"}}
                        }}
                      }}) {{
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
                data:{{ childOpt: {{ delete: {{ non_unique: "1" }} }} }}
              ) {{
                childOpt {{
                  c
                }}
              }}
            }}"#
            ),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested delete on one-to-one relation 'ChildToParent'."
        );

        Ok(())
    }

    // ----------------------------------
    // ------------ PM to C1! -----------
    // ----------------------------------

    // "a PM to C1! relation "should work through a nested mutation by id"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_by_id_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "c", c_2: "1" }},{{c: "c2", c_1: "c", c_2: "2" }}]
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
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1! relation "should work through a nested mutation by id & additional filters"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_by_id_and_fiters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "c", c_2: "1", non_unique: "0" }}, {{c: "c2", c_1: "c", c_2: "2", non_unique: "1" }}]
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
        let child = t.child().parse_extend(
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
            r#"non_unique: "0""#,
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{ delete: {child} }}
              }}
            ){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // a PM to C1!  relation should "error if the node is connected but the additional filters don't match it
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_error_if_filter_not_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "c", c_2: "1", non_unique: "0" }}, {{c: "c2", c_1: "c", c_2: "2", non_unique: "1" }}]
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
        let child = t.child().parse_extend(
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
            r#"non_unique: "2""#,
        )?;

        assert_error!(
            runner,
            format!(
                r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{ childrenOpt: {{ delete: {child} }} }}
            ) {{
              childrenOpt {{
                c
              }}
            }}
          }}"#
            ),
            2017,
            "The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected."
        );

        Ok(())
    }

    // ----------------------------------
    // ------------ P1 to C1! -----------
    // ----------------------------------

    // "a P1 to C1! relation " should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_by_id_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    // "a P1 to C1! relation " should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_by_filters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "c1", c_1: "c", c_2: "1", non_unique: "0" }}
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
              childOpt: {{ delete: {{ non_unique: "0" }} }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    // "a P1 to C1!  relation" should "error if the node is connected but the additional filters don't match it"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_error_if_filter_not_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "existingChild", c_1: "c", c_2: "1", non_unique: "0"}}
                        }}
                      }}) {{
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
                data:{{ childOpt: {{ delete: {{ non_unique: "1" }} }} }}
              ) {{
                childOpt {{
                  c
                }}
              }}
            }}"#
            ),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested delete on one-to-one relation 'ChildToParent'."
        );

        Ok(())
    }

    // ----------------------------------
    // ------------ PM to C1 ------------
    // ----------------------------------

    // "a PM to C1 " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_by_id_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1 " should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_by_id_and_filter_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "c", c_2: "1", non_unique: "0" }}, {{c: "c2", c_1: "c", c_2: "2", non_unique: "1" }}]
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
        let child = t.child().parse_extend(
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
            r#"non_unique: "0""#,
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
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // a PM to C1  relation should "error if the node is connected but the additional filters don't match it
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_error_if_filter_not_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "c1", c_1: "c", c_2: "1", non_unique: "0" }}, {{c: "c2", c_1: "c", c_2: "2", non_unique: "1" }}]
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
        let child = t.child().parse_extend(
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
            r#"non_unique: "2""#,
        )?;

        assert_error!(
            runner,
            format!(
                r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{ childrenOpt: {{ delete: {child} }} }}
            ) {{
              childrenOpt {{
                c
              }}
            }}
          }}"#
            ),
            2017,
            "The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected."
        );

        Ok(())
    }

    // ----------------------------------
    // ------------ P1! to CM -----------
    // ----------------------------------

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
            format!(
                r#"mutation {{
              updateOneParent(
              where: {parent}
              data:{{
                childReq: {{delete: true}}
              }}){{
                childReq {{
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

    // ----------------------------------
    // ------------ PM to C1 ------------
    // ----------------------------------

    // "a P1 to CM  relation " should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany", exclude(SqlServer))]
    async fn p1_cm_by_id_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation "should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany", exclude(SqlServer))]
    async fn p1_cm_by_id_and_filters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "c1", c_1: "c", c_2: "1", non_unique: "0" }}
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
              childOpt: {{ delete: {{ non_unique: "0" }} }}
            }}){{
              childOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation" should "error if the nodes are not connected"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany", exclude(SqlServer))]
    async fn p1_cm_error_if_not_connected(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
                childOpt: {{delete: true}}
              }}){{
                childOpt {{
                  c
                }}
              }}
            }}"#
            ),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested delete on one-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    // "a P1 to CM  relation" should "error if the node is connected but the additional filters don't match it"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany", exclude(SqlServer))]
    async fn p1_cm_error_if_filter_not_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "existingChild", c_1: "c", c_2: "1", non_unique: "0"}}
                        }}
                      }}) {{
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
                data:{{ childOpt: {{ delete: {{ non_unique: "1" }} }} }}
              ) {{
                childOpt {{
                  c
                }}
              }}
            }}"#
            ),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested delete on one-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    // ----------------------------------
    // ------------ PM to CM ------------
    // ----------------------------------

    // "a PM to CM  relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_by_id_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
                }}"#
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
          }}"#)),
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

    // "a PM to CM  relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_by_id_and_filters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().parse_many_first(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "otherParent", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "otherChild", c_1: "c", c_2: "1", non_unique: "0" }}]
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
                  create: [{{c: "c2", c_1: "c", c_2: "2", non_unique: "0"}},{{ c: "c3", c_1: "c", c_2: "3", non_unique: "0" }},{{c: "c4", c_1: "c", c_2: "4"}}]
                }}
              }}){{
                {selection}
              }}
            }}"#, selection = t.parent().selection())),
            &["data", "createOneParent"]
        )?;
        let child_2 = t.child().parse_extend(
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
            r#"non_unique: "0""#,
        )?;
        let child_3 = t.child().parse_extend(
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
            r#"non_unique: "1""#,
        )?;

        // Assert that nested delete is failing when child is not connected to parent (child_1 in this case)
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
                }}"#
            ),
            2017,
            "The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected."
        );

        // Assert that nested delete is failing when child is connected to parent but additional filters don't match (because of non_unique: "1")
        assert_error!(
            runner,
            format!(
                r#"mutation {{
                updateOneParent(
                  where: {parent}
                  data:{{
                    childrenOpt: {{delete: [{child_3}]}}
                }}){{
                  childrenOpt{{
                    c
                  }}
                }}
              }}"#
            ),
            2017,
            "The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected."
        );

        // Assert that nested delete works when child is connected and additional filter match
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
          }}"#)),
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
                }}"#
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
        }}"#)),
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
