use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod upsert_inside_update {
    use query_engine_tests::{run_query, run_query_json};
    use query_test_macros::relation_link_test;

    // "a PM to C1!  relation with a child already in a relation" should "work with create"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1req_child_in_req(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
              childrenOpt: {{upsert: {{
                where: {{c: "c1"}}
                update: {{c: {{ set: "updated C" }}}}
                create :{{c: "DOES NOT MATTER", c_1: "foo", c_2: "bar"}}
              }}}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"updated C"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the parent already in a relation" should "work through a nested mutation by unique for create"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_parnt_in_rel_create(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: [{{c: "c1", c_1: "foo", c_2: "bar"}}, {{c: "c2", c_1: "juuh", c_2: "lol"}}]
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
                childrenOpt: {{upsert: [{{
                  where: {{c: "DOES NOT EXIST"}}
                  update: {{c: {{set: "DOES NOT MATTER"}}}}
                  create :{{c: "new C", c_1: "omg", c_2: "lolz"}}
                }}]}}
            }}){{
              childrenOpt {{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"new C"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1  relation with the parent already in a relation" should "work through a nested mutation by unique for update"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_parnt_in_rel_update(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: [{{c: "c1", c_1: "a", c_2: "b"}}, {{c: "c2", c_1: "a2", c_2: "b2"}}]
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
          run_query!(runner,format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{upsert: [{{
                  where: {{c: "c1"}}
                  update: {{c: {{set:"updated C"}}}}
                  create :{{c: "DOES NOT MATTER", c_1: "DOES NOT MATTER", c_2: "DOES NOT MATTER"}}
                }}]}}
            }}){{
              childrenOpt (orderBy: {{ c: asc }}){{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c2"},{"c":"updated C"}]}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "work through a nested mutation by unique for update"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel_update(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: [{{c: "c1", c_1: "foo", c_2: "bar"}},{{c: "c2", c_1: "buu", c_2: "quu"}}]
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
                childrenOpt: {{
                  upsert: [{{
                    where:  {{c: "c2"}}
                    update: {{c: {{set: "updated C"}}}}
                    create: {{c: "DOES NOT MATTER", c_1: "foo", c_2: "bar"}}
                  }}]
                }}
            }}){{
              childrenOpt{{
                c
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"updated C"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"updated C","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation with the children already in a relation" should "work through a nested mutation by unique for create"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_child_inrel_create(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                        createOneParent(data: {{
                          p: "p1", p_1: "p", p_2: "1"
                          childrenOpt: {{
                            create: [{{c: "c1", c_1: "foo", c_2: "bar"}},{{c: "c2", c_1: "puu", c_2: "quu"}}]
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
              childrenOpt: {{upsert: [{{
                where: {{c: "DOES NOT EXIST"}}
                update: {{c: {{set: "DOES NOT MATTER"}}}}
                create :{{c: "updated C", c_1: "lolz", c_2: "miau"}}
              }}]}}
          }}){{
            childrenOpt{{
              c
            }}
          }}
        }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"updated C"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"updated C","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }
}
