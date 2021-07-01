use query_engine_tests::*;

#[test_suite(schema(schema))]
mod where_and_datetime {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Note{
              #id(id, String, @id, @default(cuid()))
              outerString   String
              outerDateTime DateTime @unique
              #m2m(todos, Todo[], String)
           }

           model Todo{
              #id(id, String, @id, @default(cuid()))
              innerString   String
              innerDateTime DateTime @unique
              #m2m(notes, Note[], String)
           }"#
        };

        schema.to_owned()
    }

    // "Using the same input in an update using where as used during creation of the item" should "work"
    #[connector_test]
    async fn test_1(runner: &Runner) -> TestResult<()> {
        let outer_where = r#"2018-12-05T12:34:23.000Z"#;
        let inner_where = r#"2019-12-05T12:34:23.000Z"#;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                    createOneNote(
                      data: {{
                        outerString: "Outer String"
                        outerDateTime: "{}"
                        todos: {{
                          create: [
                            {{ innerString: "Inner String", innerDateTime: "{}" }}
                          ]
                        }}
                      }}
                    ){{
                      id
                    }}
            }}"#,
                outer_where, inner_where
            )
        );

        run_query!(
            runner,
            format!(
                r#"mutation {{
                    updateOneNote(
                      where: {{ outerDateTime: "{}" }}
                      data: {{
                        outerString: {{ set: "Changed Outer String" }}
                        todos: {{
                          update: [{{
                            where: {{ innerDateTime: "{}" }},
                            data:{{ innerString: {{ set: "Changed Inner String" }} }}
                          }}]
                        }}
                      }}
                    ){{
                      id
                    }}
            }}"#,
                outer_where, inner_where
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"query{{findUniqueNote(where:{{outerDateTime: "{}" }}){{outerString, outerDateTime}} }}"#, outer_where)),
          @r###"{"data":{"findUniqueNote":{"outerString":"Changed Outer String","outerDateTime":"2018-12-05T12:34:23.000Z"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"query{{findUniqueTodo(where:{{innerDateTime: "{}" }}){{innerString, innerDateTime}} }}"#, inner_where)),
          @r###"{"data":{"findUniqueTodo":{"innerString":"Changed Inner String","innerDateTime":"2019-12-05T12:34:23.000Z"}}}"###
        );

        Ok(())
    }

    // "Using the same input in an update using where as used during creation of the item" should "work with the same time for inner and outer"
    #[connector_test]
    async fn test_2(runner: &Runner) -> TestResult<()> {
        let outer_where = r#"2018-01-03T11:27:38.000Z"#;
        let inner_where = r#"2018-01-03T11:27:38.000Z"#;

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneNote(
                    data: {{
                      outerString: "Outer String"
                      outerDateTime: "{}"
                      todos: {{
                        create: [
                          {{ innerString: "Inner String", innerDateTime: "{}" }}
                        ]
                      }}
                    }}
                  ){{
                    id
                  }}
          }}"#,
                outer_where, inner_where
            )
        );

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  updateOneNote(
                    where: {{ outerDateTime: "{}" }}
                    data: {{
                      outerString: {{ set: "Changed Outer String" }}
                      todos: {{
                        update: [{{
                          where: {{ innerDateTime: "{}" }},
                          data:{{ innerString: {{ set: "Changed Inner String" }} }}
                        }}]
                      }}
                    }}
                  ){{
                    id
                  }}
          }}"#,
                outer_where, inner_where
            )
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"query{{findUniqueNote(where:{{outerDateTime: "{}" }}){{outerString, outerDateTime}} }}"#, outer_where)),
          @r###"{"data":{"findUniqueNote":{"outerString":"Changed Outer String","outerDateTime":"2018-01-03T11:27:38.000Z"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, format!(r#"query{{findUniqueTodo(where:{{innerDateTime: "{}" }}){{innerString, innerDateTime}} }}"#, inner_where)),
          @r###"{"data":{"findUniqueTodo":{"innerString":"Changed Inner String","innerDateTime":"2018-01-03T11:27:38.000Z"}}}"###
        );

        Ok(())
    }
}
