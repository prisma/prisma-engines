use query_engine_tests::*;

#[test_suite]
mod update_inside_update {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, run_query_json, DatamodelWithParams};
    use query_test_macros::relation_link_test;

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
          }}"#, parent = parent)),
          @r###"{"data":{"updateOneParent":{"childOpt":{"non_unique":"updated"}}}}"###
        );

        Ok(())
    }

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
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"non_unique":"updated"},{"non_unique":null}]}}}"###
        );

        Ok(())
    }

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
          }}"#, parent = parent, child = child)),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"non_unique":"updated"},{"non_unique":null}]}}}"###
        );

        Ok(())
    }

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(cuid()))
              title String
              #m2m(notes, Note[], String)
             }

             model Note {
              #id(id, String, @id, @default(cuid()))
              text   String?
              #m2m(todoes, Todo[], String)
             }"#
        };

        schema.to_owned()
    }

    // Transactionality

    // "TRANSACTIONAL: a many to many relation" should "fail gracefully on wrong where and assign error correctly and not execute partially"
    #[connector_test(schema(schema_1))]
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
            }}"#, note_id = note_id),
            2016,
            "Query interpretation error. Error for binding '1': AssertionError(\"Expected a valid parent ID to be present for nested update to-one case.\")"
            // No Node for the model Todo with value DOES NOT EXIST for id found.
        );

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"query{{findUniqueNote(where:{{id: {note_id}}}){{text}}}}"#, note_id = note_id)),
          @r###"{"data":{"findUniqueNote":{"text":"Some Text"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"query{{findUniqueTodo(where:{{id: {todo_id}}}){{title}}}}"#, todo_id = todo_id)),
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
          }}"#, note_id = note_id),
          2016,
          "Query interpretation error. Error for binding '1': AssertionError(\"Expected a valid parent ID to be present for nested update to-one case.\")"
      );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Note {
              #id(id, String, @id, @default(cuid()))
              text  String? @unique
              #m2m(todos, Todo[], String)
             }

             model Todo {
              #id(id, String, @id, @default(cuid()))
              title  String  @unique
              unique String? @unique
              #m2m(notes, Note[], String)
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
            "`Mutation.updateOneNote.data.NoteUpdateInput.todos.TodoUpdateManyWithoutNotesInput.update.TodoUpdateWithWhereUniqueWithoutNotesInput.where.TodoWhereUniqueInput.unique`: A value is required but not set."
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model Top {
              #id(id, String, @id, @default(cuid()))
              nameTop String   @unique
              #m2m(middles, Middle[], String)
            }

            model Middle {
              #id(id, String, @id, @default(cuid()))
              nameMiddle String   @unique
              #m2m(tops, Top[], String)
              #m2m(bottoms, Bottom[], String)
            }

            model Bottom {
              #id(id, String, @id, @default(cuid()))
              nameBottom String   @unique
              #m2m(middles, Middle[], String)
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
