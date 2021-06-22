use query_engine_tests::*;

#[test_suite]
mod schema_gen {
    use query_engine_tests::{
        relation_field::RelationField,
        run_query, run_query_json,
        schema_with_relation::{schema_with_relation, DatamodelWithParams},
        Runner,
    };
    use query_test_macros::connector_schema_gen;

    //
    // #[connector_test(schema(generic))]
    // async fn toto(_runner: &Runner) -> TestResult<()> {
    //     let _res = schema_with_relation(
    //         RelationField::try_from("ChildOpt").unwrap(),
    //         RelationField::try_from("ParentOpt").unwrap(),
    //         false,
    //     );

    //     Ok(())
    // }

    #[connector_schema_gen(gen(ChildOpt, ParentOpt, without_params = false))]
    async fn schema_gen(runner: &Runner, params: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = params.child.qp_where.parse(
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
                    selection = params.child.selection()
                )
            ),
            &["data", "createOneParent", "childOpt"],
        );

        dbg!(&params.index, &child_1);

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
          }}"#, child = child_1)),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }
}
