use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod connect_inside_create {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams};
    use query_test_macros::relation_link_test;

    // "a P1 to C1  relation with the child already in a relation" should "be connectable through a nested mutation by id if the child is already in a relation"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_connect_by_id_already_in_rel(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                            p: "p1", p_1:"p", p_2: "1",
                            childOpt: {{
                                create: {{c: "c1", c_1:"c", c_2: "1"}}
                            }}
                        }}) {{
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
            createOneParent(data:{{
              p: "p2", p_1:"p", p_2: "2",
              childOpt: {{ connect: {child_1} }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation with the child without a relation" should "be connectable through a nested mutation by id"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_connect_by_id(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                  createOneChild(data: {{c: "c1", c_1:"c", c_2: "1"}})
                    {{
                      {selection}
                    }}
                }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            createOneParent(data:{{
              p: "p1", p_1:"p", p_2: "1",
              childOpt: {{ connect: {child_1} }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation with the child without a relation" should "be connectable through a nested mutation by id and filters"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_connect_by_id_and_filters(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().parse_extend(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                  createOneChild(data: {{c: "c1", c_1:"c", c_2: "1", non_unique: "0"}})
                    {{
                      {selection}
                    }}
                }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
            r#"non_unique: "0""#,
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            createOneParent(data:{{
              p: "p1", p_1:"p", p_2: "1",
              childOpt: {{ connect: {child_1} }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_error_if_filter_dont_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse_extend(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "c1", c_1: "foo", c_2: "bar", non_unique: "0"}})
                        {{
                          {selection}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
            r#"non_unique: "1""#,
        )?;

        assert_error!(
            runner,
            format!(
                r#"mutation {{
                createOneParent(data:{{
                  p: "p2"
                  p_1: "p2_1"
                  p_2: "p2_2"
                  childOpt: {{ connect: {child} }}
                }}){{
                  childOpt {{
                    c
                  }}
                }}
              }}"#
            ),
            2025,
            "An operation failed because it depends on one or more records that were required but not found."
        );

        Ok(())
    }

    // "a PM to C1!  relation with the child already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_to_c1_req_connect_by_uniq(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse_many_first(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1:"p", p_2: "1",
                          childrenOpt: {{
                            create: {{c: "c1", c_1:"c", c_2: "1"}}
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
            createOneParent(data:{{
              p: "p2", p_1:"p", p_2: "2",
              childrenOpt: {{ connect: {child} }}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        Ok(())
    }

    // "a P1 to C1!  relation with the child already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_to_c1_req_connect_by_uniq(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1:"p", p_2: "1",
                        childOpt: {{
                          create: {{c: "c1", c_1:"c", c_2: "1"}}
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
            createOneParent(data:{{
              p: "p2", p_1:"p", p_2: "2",
              childOpt: {{ connect: {child} }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a P1 to C1!  relation with the child already in a relation" should "be connectable through a nested mutation by unique and filters"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_to_c1_req_by_uniq_and_filters(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse_extend(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1:"p", p_2: "1",
                        childOpt: {{
                          create: {{c: "c1", c_1:"c", c_2: "1", non_unique: "0"}}
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
            r#"non_unique: "0""#,
        )?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            createOneParent(data:{{
              p: "p2", p_1:"p", p_2: "2",
              childOpt: {{ connect: {child} }}
            }}){{
              childOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the child already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_to_c1_connect_by_uniq(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "p1_1"
                  p_2: "p1_2"
                  childrenOpt: {
                    create: [{c: "c1", c_1: "foo", c_2: "bar"}, {c: "c2", c_1: "lol", c_2: "no"}]
                  }
                }){
                  childrenOpt{
                    c
                  }
                }
          }"#
        );

        // we are even resilient against multiple identical connects here -> twice connecting to c2
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data:{
              p: "p2"
              p_1: "p2_1"
              p_2: "p2_2"
              childrenOpt: {connect: [{c: "c1"}, {c: "c2"}, {c: "c2"}]}
            }){
              childrenOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the child already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_to_c1_by_uniq_and_filters(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "p1_1"
                  p_2: "p1_2"
                  childrenOpt: {
                    create: [{c: "c1", c_1: "foo", c_2: "bar", non_unique: "0"}, {c: "c2", c_1: "lol", c_2: "no", non_unique: "1"}]
                  }
                }){
                  childrenOpt{
                    c
                  }
                }
          }"#
        );

        // we are even resilient against multiple identical connects here -> twice connecting to c2
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data:{
              p: "p2"
              p_1: "p2_1"
              p_2: "p2_2"
              childrenOpt: {connect: [{c: "c1", non_unique: "0"}, {c: "c2", non_unique: "1"}, {c: "c2", non_unique: "1"}]}
            }){
              childrenOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the child without a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_to_c1_without_rel_connect_by_uniq(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1_res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                    createOneChild(data: {{c: "c1", c_1: "c", c_2: "1"}})
                    {{
                      {selection}
                    }}
                }}"#,
                selection = t.child().selection()
            )
        );
        let child_id = t.child().parse(child_1_res, &["data", "createOneChild"])?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            createOneParent(data:{{
              p: "p2"
              p_1: "p2_1"
              p_2: "p2_2"
              childrenOpt: {{ connect: {child_id} }}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1 relation with a child without a relation" should "error if also trying to connect to a non-existing node"
    // TODO: Remove when transactions are back
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_to_c1_rel_fail_connect_no_node(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "c1", c_1: "foo", c_2: "bar"}})
                        {{
                          {selection}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
        )?;

        assert_error!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data:{{
                    p: "p2"
                    p_1: "p2_1"
                    p_2: "p2_2"
                    childrenOpt: {{connect: [{child}, {{c: "DOES NOT EXIST", c_1: "no", c_2: "no"}}]}}
                  }}){{
                    childrenOpt {{
                      c
                    }}
                  }}
                }}"#
            ),
            2018,
            "The required connected records were not found. Expected 2 records to be connected after connect operation on one-to-many relation 'ChildToParent', found 1."
        );

        Ok(())
    }

    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_to_c1_rel_fail_filter_dont_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse_extend(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "c1", c_1: "foo", c_2: "bar", non_unique: "0"}})
                        {{
                          {selection}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
            r#"non_unique: "1""#,
        )?;

        assert_error!(
          runner,
          format!(
              r#"mutation {{
                createOneParent(data:{{
                  p: "p2"
                  p_1: "p2_1"
                  p_2: "p2_2"
                  childrenOpt: {{connect: [{child}]}}
                }}){{
                  childrenOpt {{
                    c
                  }}
                }}
              }}"#
          ),
          2018,
          "The required connected records were not found. Expected 1 records to be connected after connect operation on one-to-many relation 'ChildToParent', found 0."
      );

        Ok(())
    }

    // "a P1! to CM  relation with the child already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany")]
    async fn p1_req_to_cm_connect_by_uniq(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "p_1"
                  p_2: "p_2"
                  childReq: {
                    create: {
                      c: "c1"
                      c_1: "c_1"
                      c_2: "c_2"
                    }
                  }
                }){
                  childReq{
                    c
                  }
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneParent(data:{
            p: "p2"
            p_1: "1_p"
            p_2: "2_p"
            childReq: {connect: {c: "c1"}}
          }){
            childReq {
              c
            }
          }
        }"#),
          @r###"{"data":{"createOneParent":{"childReq":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation with the child not already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany")]
    async fn p1_req_to_cm_no_rel_connect_by_uniq(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneChild(data: {
                  c: "c1"
                  c_1: "c_1"
                  c_2: "c_2"
                }){
                    c
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"
            mutation {
              createOneParent(data:{
                p: "p2"
                p_1: "p_1"
                p_2: "p_2"
                childReq: {connect: {c: "c1"}}
              }){
                childReq {
                  c
                }
              }
          }"#),
          @r###"{"data":{"createOneParent":{"childReq":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyChild {
              parentsOpt { p }
            }
          }"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation with the child already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_to_cm_connect_by_uniq(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
          createOneParent(data: {
            p: "p1"
            p_1: "p1_1"
            p_2: "p1_2"
            childOpt: {
              create: {
                c: "c1"
                c_1: "c1_1"
                c_2: "c1_2"
              }
            }
          }){
            childOpt{
               c
            }
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data:{
              p: "p2"
              p_1: "p2_1"
              p_2: "p2_2"
              childOpt: {connect: {c: "c1"}}
            }){
              childOpt{
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation with the child not already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_to_cm_no_rel_connect_by_uniq(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
          createOneChild(data: {
            c: "c1"
            c_1: "c1_1"
            c_2: "c1_2"
          }){
               c
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data:{
              p: "p2"
              p_1: "p2_1"
              p_2: "p2_2"
              childOpt: {connect: {c: "c1"}}
            }){
              childOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_to_cm_connect_by_uniq(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
          createOneParent(data: {
            p: "p1"
            p_1: "p_1"
            p_2: "p_2"
            childrenOpt: {
              create: [{c: "c1", c_1: "foo", c_2: "bar"},{c: "c2", c_1: "asd", c_2: "lasd"}]
            }
          }){
            childrenOpt{
               c
            }
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data:{
              p: "p2"
              p_1: "p123"
              p_2: "p1351"
              childrenOpt: {connect: [{c: "c1"}, {c: "c2"}]}
            }){
              childrenOpt{
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the child not already in a relation" should "be connectable through a nested mutation by unique"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_to_cm_no_rel_connect_by_uniq(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneChild(data: {c: "c1", c_1: "foo", c_2: "bar"}){
                    c
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data:{
              p: "p2" p_1: "foo" p_2: "bar"
              childrenOpt: {connect: {c: "c1"}}
            }){
              childrenOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p2"}]}]}}"###
        );

        Ok(())
    }

    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_to_cm_rel_fail_filter_dont_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child = t.child().parse_extend(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneChild(data: {{c: "c1", c_1: "foo", c_2: "bar", non_unique: "0"}})
                        {{
                          {selection}
                        }}
                    }}"#,
                    selection = t.child().selection()
                )
            ),
            &["data", "createOneChild"],
            r#"non_unique: "1""#,
        )?;

        assert_error!(
          runner,
          format!(
              r#"mutation {{
                createOneParent(data:{{
                  p: "p2"
                  p_1: "p2_1"
                  p_2: "p2_2"
                  childrenOpt: {{connect: [{child}]}}
                }}){{
                  childrenOpt {{
                    c
                  }}
                }}
              }}"#
          ),
          2025,
          "An operation failed because it depends on one or more records that were required but not found. Expected 1 records to be connected, found only 0."
      );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(cuid()))
              comments Comment[]
             }

             model Comment {
              #id(id, String, @id, @default(cuid()))
              text   String
              todoId String?
              todo   Todo?   @relation(fields: todoId, references: [id])
             }"#
        };

        schema.to_owned()
    }

    // "A PM to C1 relation" should "throw a proper error if connected by wrong id"
    #[connector_test(schema(schema_2))]
    async fn pm_to_c1_fail_if_wrong_id(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
              createOneTodo(data:{
                comments: {
                  connect: [{id: "5beea4aa6183dd734b2dbd9b"}]
                }
              }){
                id
                comments {
                  id
                  text
                }
              }
            }"#,
            2018,
            "The required connected records were not found. Expected 1 records to be connected after connect operation on one-to-many relation 'CommentToTodo', found 0."
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model Comment {
            #id(id, String, @id, @default(cuid()))
            text   String
            todoId String?
            todo   Todo?   @relation(fields: todoId, references: [id])
           }

           model Todo {
            #id(id, String, @id, @default(cuid()))
            text     String?
            comments Comment[]
           }"#
        };

        schema.to_owned()
    }

    // "A P1 to CM relation " should "throw a proper error if connected by wrong id the other way around"
    #[connector_test(schema(schema_3))]
    async fn p1_to_cm_fail_if_wrong_id_other_side(runner: Runner) -> TestResult<()> {
        assert_error!(
          &runner,
          r#"mutation {
            createOneComment(data:{
              text: "bla"
              todo: {
                connect: {id: "5beea4aa6183dd734b2dbd9b"}
              }
            }){
              id
            }
          }"#,
          2025,
          "An operation failed because it depends on one or more records that were required but not found. No 'Todo' record(s) (needed to inline the relation on 'Comment' record(s)) was found for a nested connect on one-to-many relation 'CommentToTodo'"
      );

        Ok(())
    }
}
