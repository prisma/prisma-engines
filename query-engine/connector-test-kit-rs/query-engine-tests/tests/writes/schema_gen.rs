use query_engine_tests::*;

#[test_suite]
mod schema_gen {
    use query_engine_tests::{
        run_query, run_query_json, schema_with_relation::DatamodelWithParams, ConnectorCapability, Runner,
    };
    use query_test_macros::connector_schema_gen;

    #[connector_schema_gen(gen(ChildOpt, ParentOpt))]
    async fn schema_gen(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        let child_1 = t.child().wher().parse(
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
