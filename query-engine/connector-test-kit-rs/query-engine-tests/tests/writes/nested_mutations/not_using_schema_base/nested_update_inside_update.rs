use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod update_inside_update {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams};
    use query_test_macros::relation_link_test;

    // ----------------------------------
    // ------------ P1 to CM ------------
    // ----------------------------------

    // "A P1 to CM relation relation" should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
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
                childOpt: {{
                  update: {{ non_unique: {{ set: "updated" }}}}
                }}
            }}){{
              childOpt {{
                non_unique
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"non_unique":"updated"}}}}"###
        );

        Ok(())
    }

    // "A P1 to CM relation relation" should "work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm_by_id_and_filters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childOpt: {{
                          create: {{c: "c1", c_1: "c", c_2: "1", non_unique: "0"}}
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
                childOpt: {{
                  update: {{ where: {{ non_unique: "0" }} data: {{ non_unique: {{ set: "updated" }} }} }}
                }}
            }}){{
              childOpt {{
                non_unique
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"non_unique":"updated"}}}}"###
        );

        Ok(())
    }

    // "a P1 to CM relation" should "error if the nodes are not connected"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
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
            format!(r#"mutation {{
              updateOneParent(
              where: {parent}
              data:{{
                childOpt: {{update: {{ non_unique: "updated" }}}}
              }}){{
                childOpt {{
                  c
                }}
              }}
            }}"#),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested update on one-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    // "a P1 to CM relation" should "error if the node is connected but the additional filters don't match it"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
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
            format!(r#"mutation {{
              updateOneParent(
                where: {parent}
                data:{{ childOpt: {{ update: {{ where: {{ non_unique: "1" }}, data: {{ non_unique: "updated" }} }} }} }}
              ) {{
                childOpt {{
                  c
                }}
              }}
            }}"#),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested update on one-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    // ----------------------------------
    // ------------ PM to C1 ------------
    // ----------------------------------

    // "A PM to C1 relation relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1",
                    childrenOpt: {{
                      create: [{{c: "c1", c_1: "c", c_2: "1"}},{{c: "c2", c_1: "c", c_2: "2"}}]
                    }}
                  }}) {{
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
        let child = t
            .child()
            .parse_many_first(res, &["data", "createOneParent", "childrenOpt"])?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{
                    update:  [
                      {{ where: {child}, data: {{ non_unique: {{ set: "updated" }} }}}}
                    ]
                  }}
            }}){{
              childrenOpt (orderBy: {{ c: asc }} ){{
                non_unique
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"non_unique":"updated"},{"non_unique":null}]}}}"###
        );

        Ok(())
    }

    // "A PM to C1 relation relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_by_id_and_filters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1",
                    childrenOpt: {{
                      create: [{{c: "c1", c_1: "c", c_2: "1", non_unique: "0"}},{{c: "c2", c_1: "c", c_2: "2", non_unique: "1"}}]
                    }}
                  }}) {{
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

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{
                    update:  [
                      {{ where: {{ c: "c1", non_unique: "0" }}, data: {{ non_unique: {{ set: "updated" }} }}}}
                    ]
                  }}
            }}){{
              childrenOpt (orderBy: {{ c: asc }} ){{
                non_unique
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"non_unique":"updated"},{"non_unique":"1"}]}}}"###
        );

        Ok(())
    }

    // "a PM to C1 relation" should "error if the nodes are not connected"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_error_if_not_connected(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
                data: {{
                  childrenOpt: {{
                    update: {{
                      where: {{ c: "c1" }},
                      data: {{ non_unique: {{ set: "updated" }} }}
                    }}
                  }}
              }}){{
                childrenOpt {{
                  c
                }}
              }}
            }}"#),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested update on one-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    // "a PM to C1 relation" should "error if the node is connected but the additional filters don't match it"
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
                          create: [{{c: "existingChild", c_1: "c", c_2: "1", non_unique: "0"}}]
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
            format!(r#"mutation {{
              updateOneParent(
                where: {parent}
                data:{{ childrenOpt: {{
                  update: {{
                    where: {{ c: "existingChild", non_unique: "1" }},
                    data: {{ non_unique: "updated" }} }}
                  }}
                }}
              ) {{
                childrenOpt {{
                  c
                }}
              }}
            }}"#),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested update on one-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    // ----------------------------------
    // ------------ PM to CM ------------
    // ----------------------------------

    // "A PM to CM relation relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
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
        let child = t
            .child()
            .parse_many_first(res, &["data", "createOneParent", "childrenOpt"])?;

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{
                    update:  [
                      {{where: {child}, data: {{non_unique: {{ set: "updated" }}}}}}
                    ]
                  }}
            }}){{
              childrenOpt (orderBy: {{ c: asc }} ){{
                non_unique
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"non_unique":"updated"},{"non_unique":null}]}}}"###
        );

        Ok(())
    }

    // "A PM to CM relation relation" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_by_id_and_filters_should_work(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  createOneParent(data: {{
                    p: "p1", p_1: "p", p_2: "1",
                    childrenOpt: {{
                      create: [{{c: "c1", c_1: "c", c_2: "1", non_unique: "0"}},{{c: "c2", c_1: "c", c_2: "2", non_unique: "1"}}]
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

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"mutation {{
            updateOneParent(
              where: {parent}
              data:{{
                childrenOpt: {{
                    update:  [
                      {{where: {{ c: "c1", non_unique: "0" }}, data: {{non_unique: {{ set: "updated" }}}}}}
                    ]
                  }}
            }}){{
              childrenOpt (orderBy: {{ c: asc }} ){{
                non_unique
              }}
            }}
          }}"#)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"non_unique":"updated"},{"non_unique":"1"}]}}}"###
        );

        Ok(())
    }

    // "a PM to CM relation" should "error if the nodes are not connected"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_error_if_not_connected(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
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
                data: {{
                  childrenOpt: {{
                    update: {{
                      where: {{ c: "c1" }},
                      data: {{ non_unique: {{ set: "updated" }} }}
                    }}
                  }}
              }}){{
                childrenOpt {{
                  c
                }}
              }}
            }}"#),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested update on many-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    // "a PM to CM relation" should "error if the node is connected but the additional filters don't match it"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_error_if_filter_not_match(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let parent = t.parent().parse(
            run_query_json!(
                runner,
                format!(
                    r#"mutation {{
                      createOneParent(data: {{
                        p: "p1", p_1: "p", p_2: "1",
                        childrenOpt: {{
                          create: [{{c: "existingChild", c_1: "c", c_2: "1", non_unique: "0"}}]
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
            format!(r#"mutation {{
              updateOneParent(
                where: {parent}
                data:{{ childrenOpt: {{
                  update: {{
                    where: {{ c: "existingChild", non_unique: "1" }},
                    data: {{ non_unique: "updated" }} }}
                  }}
                }}
              ) {{
                childrenOpt {{
                  c
                }}
              }}
            }}"#),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record was found for a nested update on many-to-many relation 'ChildToParent'."
        );

        Ok(())
    }

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(cuid()))
              title String
              #m2m(notes, Note[], id, String)
             }

             model Note {
              #id(id, String, @id, @default(cuid()))
              text   String?
              #m2m(todoes, Todo[], id, String)
             }"#
        };

        schema.to_owned()
    }

    // Transactionality

    #[connector_test(schema(schema_1), exclude(Sqlite("cfd1")))]
    // "TRANSACTIONAL: a many to many relation" should "fail gracefully on wrong where and assign error correctly and not execute partially"
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"findUniqueNote":{"text":"Some Text"}}}
    // + {"data":{"findUniqueNote":{"text":"Some Changed Text"}}}
    // ```
    async fn tx_m2m_fail_wrong_where(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
          createOneNote(
            data: {
              text: "Some Text"
              todoes: {
                create: { title: "the title" }
              }
            }
          ){
            id
            todoes { id }
          }
        }"#
        );
        let note_id = &res["data"]["createOneNote"]["id"].to_string();
        let todo_id = &res["data"]["createOneNote"]["todoes"][0]["id"].to_string();

        assert_error!(
            runner,
            format!(r#"mutation {{
              updateOneNote(
                where: {{
                  id: {note_id}
                }}
                data: {{
                  text: {{ set: "Some Changed Text" }}
                  todoes: {{
                    update: {{
                      where: {{id: "DOES NOT EXIST"}},
                      data:{{ title: {{ set: "updated title" }}}}
                    }}
                  }}
                }}
              ){{
                text
              }}
            }}"#),
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Todo' record was found for a nested update on many-to-many relation 'NoteToTodo'."
            // No Node for the model Todo with value DOES NOT EXIST for id found.
        );

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"query{{findUniqueNote(where:{{id: {note_id}}}){{text}}}}"#)),
          @r###"{"data":{"findUniqueNote":{"text":"Some Text"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"query{{findUniqueTodo(where:{{id: {todo_id}}}){{title}}}}"#)),
          @r###"{"data":{"findUniqueTodo":{"title":"the title"}}}"###
        );

        Ok(())
    }

    // "NON-TRANSACTIONAL: a many to many relation" should "fail gracefully on wrong where and assign error correctly and not execute partially"
    #[connector_test(schema(schema_1))]
    async fn no_tx_m2m_fail_gracefully(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
                createOneNote(
                  data: {
                    text: "Some Text"
                    todoes: {
                      create: { title: "the title" }
                    }
                  }
                ){
                  id
                  todoes { id }
                }
            }"#
        );
        let note_id = &res["data"]["createOneNote"]["id"].to_string();

        assert_error!(
          runner,
          format!(r#"mutation {{
            updateOneNote(
              where: {{
                id: {note_id}
              }}
              data: {{
                text: {{ set: "Some Changed Text" }}
                todoes: {{
                  update: {{
                    where: {{id: "5beea4aa6183dd734b2dbd9b"}},
                    data:{{ title: {{ set: "updated title" }}}}
                  }}
                }}
              }}
            ){{
              text
            }}
          }}"#),
          2025,
          "An operation failed because it depends on one or more records that were required but not found. No 'Todo' record was found for a nested update on many-to-many relation 'NoteToTodo'."
      );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Note {
              #id(id, String, @id, @default(cuid()))
              text  String? @unique
              #m2m(todos, Todo[], id, String)
             }

             model Todo {
              #id(id, String, @id, @default(cuid()))
              title  String  @unique
              unique String? @unique
              #m2m(notes, Note[], id, String)
             }"#
        };

        schema.to_owned()
    }

    // "a many to many relation" should "reject null in unique fields"
    #[connector_test(schema(schema_2))]
    async fn m2m_reject_null_in_uniq(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
              createOneNote(
                data: {
                  text: "Some Text"
                  todos: {
                  create: [{ title: "the title", unique: "test"}, { title: "the other title" }]
                  }
                }
              ){
                id
                todos { id }
              }
            }"#
        );

        assert_error!(
            &runner,
            r#"mutation {
              updateOneNote(
                where: {
                  text: "Some Text"
                }
                data: {
                  text: { set: "Some Changed Text" }
                  todos: {
                    update: {
                      where: { unique: null },
                      data: { title: { set: "updated title" }}
                    }
                  }
                }
              ){
                text
                todos {
                  title
                }
              }
            }"#,
            2009, // 3040
            "A value is required but not set"
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model Top {
              #id(id, String, @id, @default(cuid()))
              nameTop String   @unique
              #m2m(middles, Middle[], id, String)
            }

            model Middle {
              #id(id, String, @id, @default(cuid()))
              nameMiddle String   @unique
              #m2m(tops, Top[], id, String)
              #m2m(bottoms, Bottom[], id, String)
            }

            model Bottom {
              #id(id, String, @id, @default(cuid()))
              nameBottom String   @unique
              #m2m(middles, Middle[], id, String)
            }"#
        };

        schema.to_owned()
    }

    // "a deeply nested mutation" should "execute all levels of the mutation if there are only node edges on the path"
    #[connector_test(schema(schema_3))]
    async fn deep_nested_mutation_exec_all_muts(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation  {
          createOneTop(data: {
            nameTop: "the top",
            middles: {
              create:[
                {
                  nameMiddle: "the middle"
                  bottoms: {
                    create: [{ nameBottom: "the bottom"}, { nameBottom: "the second bottom"}]
                  }
                },
                {
                  nameMiddle: "the second middle"
                  bottoms: {
                    create: [{nameBottom: "the third bottom"},{nameBottom: "the fourth bottom"}]
                  }
                }
             ]
            }
          }) {id}
        }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation b {
            updateOneTop(
              where: {nameTop: "the top"},
              data: {
                nameTop: { set: "updated top" }
                middles: {
                  update: [{
                        where: { nameMiddle: "the middle" },
                        data:{
                          nameMiddle: { set: "updated middle" }
                          bottoms: {
                            update: [{
                              where: { nameBottom: "the bottom" },
                              data:  { nameBottom: { set: "updated bottom" }}
                            }]
                        }
                      }
                    }
                  ]
               }
             }
            ) {
              nameTop
              middles (orderBy: { id: asc }){
                nameMiddle
                bottoms (orderBy: { id: asc }){
                  nameBottom
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottoms":[{"nameBottom":"updated bottom"},{"nameBottom":"the second bottom"}]},{"nameMiddle":"the second middle","bottoms":[{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}]}}}"###
        );

        Ok(())
    }
}
