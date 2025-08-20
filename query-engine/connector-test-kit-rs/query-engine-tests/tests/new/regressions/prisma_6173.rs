use query_engine_tests::*;

#[test_suite(schema(dummy))]
mod query_raw {

    fn dummy() -> String {
        let schema = indoc! {r#"
        model Test {
            id  Int @id
        }
        "#};

        schema.to_string()
    }

    // MariaDB is the only one supporting anon blocks
    #[connector_test(only(MySQL("mariadb")))]
    async fn mysql_call(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"
              mutation {
                  queryRaw(
                      query: "BEGIN NOT ATOMIC\n INSERT INTO Test VALUES(FLOOR(RAND()*1000));\n SELECT * FROM Test;\n END",
                      parameters: "[]"
                  )
              }
            "#
        );
        // fmt_execute_raw cannot run this query, doing it directly instead
        insta::assert_json_snapshot!(res,
        {
            ".data.queryRaw.rows[0][0]" => "<rand_int>"
        }, @r###"
        {
          "data": {
            "queryRaw": {
              "columns": [
                "id"
              ],
              "types": [
                "int"
              ],
              "rows": [
                [
                  "<rand_int>"
                ]
              ]
            }
          }
        }
        "###);

        Ok(())
    }

    #[connector_test(only(MySQL("mariadb")))]
    async fn mysql_call_2(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"
              mutation {
                  queryRaw(
                      query: "BEGIN NOT ATOMIC\n INSERT INTO Test VALUES(FLOOR(RAND()*1000));\n SELECT * FROM Test WHERE 1=0;\n END",
                      parameters: "[]"
                  )
              }
            "#
        );

        insta::assert_json_snapshot!(res,
          @r###"
        {
          "data": {
            "queryRaw": {
              "columns": [],
              "types": [],
              "rows": []
            }
          }
        }
        "###);

        Ok(())
    }
}
