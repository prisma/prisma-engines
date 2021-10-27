use query_engine_tests::*;

#[test_suite(schema(common_list_types), capabilities(ScalarLists))]
mod lists {
    use indoc::indoc;
    use query_engine_tests::run_query;

    #[connector_test]
    async fn equality(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "string", "equals", r#"["a", "A", "c"]"#, vec![1]).await?;
        list_query(&runner, "int", "equals", r#"[1, 2, 3]"#, vec![1]).await?;
        list_query(&runner, "float", "equals", r#"[1.1, 2.2, 3.3]"#, vec![1]).await?;
        list_query(&runner, "bInt", "equals", r#"["100", "200", "300"]"#, vec![1]).await?;
        list_query(&runner, "decimal", "equals", r#"["11.11", "22.22", "33.33"]"#, vec![1]).await?;
        list_query(&runner, "bool", "equals", r#"[true]"#, vec![1]).await?;
        list_query(&runner, "bytes", "equals", r#"["dGVzdA==", "dA=="]"#, vec![1]).await?;
        list_query(
            &runner,
            "dt",
            "equals",
            r#"["1969-01-01T10:33:59.000Z", "2018-12-05T12:34:23.000Z"]"#,
            vec![1],
        )
        .await?;

        Ok(())
    }

    #[connector_test]
    async fn has(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "string", "has", r#""A""#, vec![1]).await?;
        list_query(&runner, "int", "has", "2", vec![1]).await?;
        list_query(&runner, "float", "has", "1.1", vec![1]).await?;
        list_query(&runner, "bInt", "has", r#""200""#, vec![1]).await?;
        list_query(&runner, "decimal", "has", "33.33", vec![1]).await?;
        list_query(&runner, "dt", "has", r#""2018-12-05T12:34:23.000Z""#, vec![1]).await?;
        list_query(&runner, "bool", "has", "true", vec![1]).await?;
        list_query(&runner, "bytes", "has", r#""dGVzdA==""#, vec![1]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "string", "hasSome", r#"["A", "c"]"#, vec![1]).await?;
        list_query(&runner, "int", "hasSome", r#"[2, 10]"#, vec![1]).await?;
        list_query(&runner, "float", "hasSome", r#"[1.1, 5.5]"#, vec![1]).await?;
        list_query(&runner, "bInt", "hasSome", r#"["200", "5000"]"#, vec![1]).await?;
        list_query(&runner, "decimal", "hasSome", r#"[55.55, 33.33]"#, vec![1]).await?;
        list_query(&runner, "bool", "hasSome", r#"[true, false]"#, vec![1]).await?;
        list_query(&runner, "string", "hasSome", r#"[]"#, vec![]).await?;

        list_query(
            &runner,
            "dt",
            "hasSome",
            r#"["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]"#,
            vec![1],
        )
        .await?;

        list_query(
            &runner,
            "bytes",
            "hasSome",
            r#"["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]"#,
            vec![1],
        )
        .await?;

        Ok(())
    }

    #[connector_test]
    async fn has_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "string", "hasEvery", r#"["A", "d"]"#, vec![]).await?;
        list_query(&runner, "string", "hasEvery", r#"["A"]"#, vec![1]).await?;

        list_query(&runner, "int", "hasEvery", r#"[2, 10]"#, vec![]).await?;
        list_query(&runner, "int", "hasEvery", r#"[2]"#, vec![1]).await?;

        list_query(&runner, "float", "hasEvery", r#"[1.1, 5.5]"#, vec![]).await?;
        list_query(&runner, "float", "hasEvery", r#"[1.1]"#, vec![1]).await?;

        list_query(&runner, "bInt", "hasEvery", r#"["200", "5000"]"#, vec![]).await?;
        list_query(&runner, "bInt", "hasEvery", r#"["200"]"#, vec![1]).await?;

        list_query(&runner, "decimal", "hasEvery", r#"[55.55, 33.33]"#, vec![]).await?;
        list_query(&runner, "decimal", "hasEvery", r#"[33.33]"#, vec![1]).await?;

        list_query(&runner, "dt", "hasEvery", r#"["2018-12-05T12:34:23.000Z"]"#, vec![1]).await?;
        list_query(
            &runner,
            "dt",
            "hasEvery",
            r#"["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]"#,
            vec![],
        )
        .await?;

        list_query(&runner, "bool", "hasEvery", r#"[true, false]"#, vec![]).await?;
        list_query(&runner, "bool", "hasEvery", r#"[true]"#, vec![1]).await?;

        list_query(&runner, "bytes", "hasEvery", r#"["dGVzdA=="]"#, vec![1]).await?;
        list_query(
            &runner,
            "bytes",
            "hasEvery",
            r#"["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]"#,
            vec![],
        )
        .await?;

        Ok(())
    }

    #[connector_test]
    async fn is_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "string", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "int", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "float", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "bInt", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "decimal", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "dt", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "bool", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "bytes", "isEmpty", "true", vec![2]).await?;

        list_query(&runner, "string", "isEmpty", "false", vec![1]).await?;
        list_query(&runner, "int", "isEmpty", "false", vec![1]).await?;
        list_query(&runner, "float", "isEmpty", "false", vec![1]).await?;
        list_query(&runner, "bInt", "isEmpty", "false", vec![1]).await?;
        list_query(&runner, "decimal", "isEmpty", "false", vec![1]).await?;
        list_query(&runner, "dt", "isEmpty", "false", vec![1]).await?;
        list_query(&runner, "bool", "isEmpty", "false", vec![1]).await?;
        list_query(&runner, "bytes", "isEmpty", "false", vec![1]).await?;

        list_query(&runner, "string", "hasSome", "[]", vec![]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has_every_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { hasEvery: [] }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc::indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:      1,
                  string:  ["a", "A", "c"],
                  int:     [1, 2, 3],
                  float:   [1.1, 2.2, 3.3],
                  bInt:    ["100", "200", "300"],
                  decimal: ["11.11", "22.22", "33.33"],
                  dt:      ["1969-01-01T10:33:59.000Z", "2018-12-05T12:34:23.000Z"],
                  bool:    [true],
                  bytes:   ["dGVzdA==", "dA=="],
                }) { id }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:      2,
                  string:  [],
                  int:     [],
                  float:   [],
                  bInt:    [],
                  decimal: [],
                  dt:      [],
                  bool:    [],
                  bytes:   []
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}

// CockroachDB cannot store Json[], but can process them in memory.
// See https://github.com/cockroachdb/cockroach/issues/23468.
#[test_suite(schema(schema), capabilities(ScalarLists, Json), exclude(Cockroach))]
mod json_lists {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              json Json[]
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn equality(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(
            &runner,
            "json",
            "equals",
            r#"["{}", "{\"int\":5}", "[1, 2, 3]"]"#,
            vec![1],
        )
        .await?;

        list_query(&runner, "json", "equals", r#"["null", "\"test\""]"#, vec![3]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "json", "has", r#""[1, 2, 3]""#, vec![1]).await?;
        list_query(&runner, "json", "has", r#""null""#, vec![3]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "json", "hasSome", r#"["{}", "[1]"]"#, vec![1]).await?;
        list_query(&runner, "json", "hasSome", r#"["null", "\"test 2\""]"#, vec![3]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "json", "hasEvery", r#"["{}", "[1]"]"#, vec![]).await?;
        list_query(&runner, "json", "hasEvery", r#"["{}"]"#, vec![1]).await?;
        list_query(&runner, "json", "hasEvery", r#"["null"]"#, vec![3]).await?;

        Ok(())
    }

    #[connector_test]
    async fn is_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "json", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "json", "isEmpty", "false", vec![1, 3]).await?;

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc::indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   1,
                  json: ["{}", "{\"int\":5}", "[1, 2, 3]"]
                }) { id }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   2,
                  json: []
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   3,
                  json: ["null", "\"test\""]
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}

#[test_suite(schema(schema), capabilities(ScalarLists, Enums))]
mod enum_lists {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model TestModel {
              #id(id, Int, @id)
              enum TestEnum[]
            }

            enum TestEnum {
                A
                B
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn equality(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;
        list_query(&runner, "enum", "equals", r#"[A, B, B, A]"#, vec![1]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;
        list_query(&runner, "enum", "has", "A", vec![1]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;
        list_query(&runner, "enum", "hasSome", r#"[A]"#, vec![1]).await?;

        Ok(())
    }

    #[connector_test]
    async fn has_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;
        list_query(&runner, "enum", "hasEvery", r#"[A, B]"#, vec![1]).await?;

        Ok(())
    }

    #[connector_test]
    async fn is_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        list_query(&runner, "enum", "isEmpty", "true", vec![2]).await?;
        list_query(&runner, "enum", "isEmpty", "false", vec![1]).await?;

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc::indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   1,
                  enum: [A, B, B, A]
                }) { id }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   2,
                  enum: [],
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}

async fn list_query(
    runner: &Runner,
    field: &str,
    operation: &str,
    comparator: &str,
    expected_ids: Vec<i32>,
) -> TestResult<()> {
    let result = runner
        .query(format!(
            indoc::indoc! { r#"
                query {{
                  findManyTestModel(where: {{
                    {}: {{ {}: {} }}
                  }}) {{
                    id
                  }}
                }}
                "#},
            field, operation, comparator
        ))
        .await?;

    result.assert_success();

    if expected_ids.is_empty() {
        assert_eq!(result.to_string(), r#"{"data":{"findManyTestModel":[]}}"#);
    } else {
        let stringified: Vec<_> = expected_ids
            .into_iter()
            .map(|id| format!(r#"{{"id":{}}}"#, id))
            .collect();

        assert_eq!(
            result.to_string(),
            format!(r#"{{"data":{{"findManyTestModel":[{}]}}}}"#, stringified.join(","))
        )
    }

    Ok(())
}
