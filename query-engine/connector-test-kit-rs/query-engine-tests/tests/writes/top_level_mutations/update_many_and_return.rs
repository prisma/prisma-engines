use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(UpdateReturning))]
mod update_many_and_return {
    use indoc::indoc;
    use query_engine_tests::{is_one_of, run_query, run_query_json};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int,  @id)
              optStr   String?
              optInt   Int?
              optFloat Float?
            }"#
        };

        schema.to_owned()
    }

    // "An updateManyAndReturn mutation" should "update the records matching the where clause"
    #[connector_test]
    async fn update_recs_matching_where(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(&runner, r#"{ id: 2, optStr: "str2" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateManyTestModelAndReturn(
              where: { optStr: { equals: "str1" } }
              data: { optStr: { set: "str1new" }, optInt: { set: 1 }, optFloat: { multiply: 2 } }
            ) {
              optStr optInt optFloat
            }
          }"#),
          @r###"{"data":{"updateManyTestModelAndReturn":[{"optStr":"str1new","optInt":1,"optFloat":null}]}}"###
        );

        Ok(())
    }

    // "An updateMany mutation" should "update the records matching the where clause using shorthands"
    #[connector_test]
    async fn update_recs_matching_where_shorthands(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(&runner, r#"{ id: 2, optStr: "str2" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateManyTestModelAndReturn(
              where: { optStr: "str1" }
              data: { optStr: "str1new", optInt: null, optFloat: { multiply: 2 } }
            ) {
              optStr optInt optFloat
            }
          }"#),
          @r###"{"data":{"updateManyTestModelAndReturn":[{"optStr":"str1new","optInt":null,"optFloat":null}]}}"###
        );

        Ok(())
    }

    // "An updateManyAndReturn mutation" should "update all items if the where clause is empty"
    #[connector_test]
    async fn update_all_items_if_where_empty(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 2, optStr: "str2", optInt: 2 }"#).await?;
        create_row(&runner, r#"{ id: 3, optStr: "str3", optInt: 3, optFloat: 3.1 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateManyTestModelAndReturn(
              where: { }
              data: { optStr: { set: "updated" }, optFloat: { divide: 2 }, optInt: { decrement: 1 } }
            ){
              optStr optInt optFloat
            }
          }"#),
          @r###"{"data":{"updateManyTestModelAndReturn":[{"optStr":"updated","optInt":1,"optFloat":null},{"optStr":"updated","optInt":2,"optFloat":1.55}]}}"###
        );

        Ok(())
    }

    // "An updateManyAndReturn mutation" should "correctly apply all number operations for Int"
    #[connector_test(exclude(CockroachDb))]
    async fn apply_number_ops_for_int(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(&runner, r#"{ id: 2, optStr: "str2", optInt: 2 }"#).await?;
        create_row(&runner, r#"{ id: 3, optStr: "str3", optInt: 3, optFloat: 3.1 }"#).await?;

        is_one_of!(
            query_number_operation(&runner, "optInt", "increment", "10").await?,
            [
                r#"[{"optInt":null},{"optInt":12},{"optInt":13}]"#,
                r#"[{"optInt":10},{"optInt":12},{"optInt":13}]"#
            ]
        );

        // optInts before this op are now: null/10, 12, 13
        is_one_of!(
            query_number_operation(&runner, "optInt", "decrement", "10").await?,
            [
                r#"[{"optInt":null},{"optInt":2},{"optInt":3}]"#,
                r#"[{"optInt":0},{"optInt":2},{"optInt":3}]"#
            ]
        );

        // optInts before this op are now: null/0, 2, 3
        is_one_of!(
            query_number_operation(&runner, "optInt", "multiply", "2").await?,
            [
                r#"[{"optInt":null},{"optInt":4},{"optInt":6}]"#,
                r#"[{"optInt":0},{"optInt":4},{"optInt":6}]"#
            ]
        );

        // Todo: Mongo divisions are broken
        if !matches!(runner.connector_version(), ConnectorVersion::MongoDb(_)) {
            // optInts before this op are now: null/0, 4, 6
            is_one_of!(
                query_number_operation(&runner, "optInt", "divide", "3").await?,
                [
                    r#"[{"optInt":null},{"optInt":1},{"optInt":2}]"#,
                    r#"[{"optInt":0},{"optInt":1},{"optInt":2}]"#
                ]
            );
        }

        is_one_of!(
            query_number_operation(&runner, "optInt", "set", "5").await?,
            [
                r#"[{"optInt":5},{"optInt":5},{"optInt":5}]"#,
                r#"[{"optInt":5},{"optInt":5},{"optInt":5}]"#
            ]
        );

        is_one_of!(
            query_number_operation(&runner, "optInt", "set", "null").await?,
            [
                r#"[{"optInt":null},{"optInt":null},{"optInt":null}]"#,
                r#"[{"optInt":null},{"optInt":null},{"optInt":null}]"#
            ]
        );

        Ok(())
    }

    // CockroachDB does not support the "divide" operator as is.
    // See https://github.com/cockroachdb/cockroach/issues/41448.
    #[connector_test(only(CockroachDb))]
    async fn apply_number_ops_for_int_cockroach(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(&runner, r#"{ id: 2, optStr: "str2", optInt: 2 }"#).await?;
        create_row(&runner, r#"{ id: 3, optStr: "str3", optInt: 3, optFloat: 3.1 }"#).await?;

        is_one_of!(
            query_number_operation(&runner, "optInt", "increment", "10").await?,
            [
                r#"[{"optInt":null},{"optInt":12},{"optInt":13}]"#,
                r#"[{"optInt":10},{"optInt":12},{"optInt":13}]"#
            ]
        );

        // optInts before this op are now: null/10, 12, 13
        is_one_of!(
            query_number_operation(&runner, "optInt", "decrement", "10").await?,
            [
                r#"[{"optInt":null},{"optInt":2},{"optInt":3}]"#,
                r#"[{"optInt":0},{"optInt":2},{"optInt":3}]"#
            ]
        );

        // optInts before this op are now: null/0, 2, 3
        is_one_of!(
            query_number_operation(&runner, "optInt", "multiply", "2").await?,
            [
                r#"[{"optInt":null},{"optInt":4},{"optInt":6}]"#,
                r#"[{"optInt":0},{"optInt":4},{"optInt":6}]"#
            ]
        );

        is_one_of!(
            query_number_operation(&runner, "optInt", "set", "5").await?,
            [
                r#"[{"optInt":5},{"optInt":5},{"optInt":5}]"#,
                r#"[{"optInt":5},{"optInt":5},{"optInt":5}]"#
            ]
        );

        is_one_of!(
            query_number_operation(&runner, "optInt", "set", "null").await?,
            [
                r#"[{"optInt":null},{"optInt":null},{"optInt":null}]"#,
                r#"[{"optInt":null},{"optInt":null},{"optInt":null}]"#
            ]
        );

        Ok(())
    }

    // "An updateManyAndReturn mutation" should "correctly apply all number operations for Float"
    #[connector_test]
    async fn apply_number_ops_for_float(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(&runner, r#"{ id: 2, optStr: "str2", optFloat: 2 }"#).await?;
        create_row(&runner, r#"{ id: 3, optStr: "str3", optFloat: 3.1 }"#).await?;

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "increment", "1.1").await?,
          @r###"[{"optFloat":null},{"optFloat":3.1},{"optFloat":4.2}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "decrement", "1.1").await?,
          @r###"[{"optFloat":null},{"optFloat":2},{"optFloat":3.1}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "multiply", "5.5").await?,
          @r###"[{"optFloat":null},{"optFloat":11},{"optFloat":17.05}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "divide", "2").await?,
          @r###"[{"optFloat":null},{"optFloat":5.5},{"optFloat":8.525}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "set", "5").await?,
          @r###"[{"optFloat":5},{"optFloat":5},{"optFloat":5}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "set", "null").await?,
          @r###"[{"optFloat":null},{"optFloat":null},{"optFloat":null}]"###
        );

        Ok(())
    }

    fn schema_11_child() -> String {
        let schema = indoc! {
            r#"model Test {
                  #id(id, Int, @id)

                  child Child?
                }

                model Child {
                  #id(id, Int, @id)

                  testId Int? @unique
                  test Test? @relation(fields: [testId], references: [id])

                }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_1m_child))]
    async fn update_many_11_inline_rel_read_works(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner,r#"mutation { createManyTest(data: [{ id: 1 }, { id: 2 }]) { count } }"#),
            @r###"{"data":{"createManyTest":{"count":2}}}"###
        );
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createManyChild(data: [{ id: 1, testId: 1 }, { id: 2, testId: 2 }]) { count } }"#),
            @r###"{"data":{"createManyChild":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
                updateManyChildAndReturn(
                    where: {}
                    data: { testId: 1 }
                ) { id test { id } }
              }"#),
          @r###"{"data":{"updateManyChildAndReturn":[{"id":1,"test":{"id":1}},{"id":2,"test":{"id":1}}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_11_child))]
    async fn update_many_11_non_inline_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Test",
              "action": "updateManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "child": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'child' not found in enclosing type".to_string()));

        Ok(())
    }

    fn schema_1m_child() -> String {
        let schema = indoc! {
            r#"model Test {
                  #id(id, Int, @id)
                  str1 String?
                  str2 String?
                  str3 String? @default("SOME_DEFAULT")

                  children Child[]
                }

                model Child {
                  #id(id, Int, @id)
                  str1 String?
                  str2 String?
                  str3 String? @default("SOME_DEFAULT")

                  testId Int?
                  test Test? @relation(fields: [testId], references: [id])

                }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_1m_child))]
    async fn update_many_1m_inline_rel_read_works(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createManyTest(data: [{ id: 1, str1: "1" }, { id: 2, str1: "2" }]) { count } }"#),
          @r###"{"data":{"createManyTest":{"count":2}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createManyChild(data: [{ id: 1, str1: "1", testId: 1 }, { id: 2, str1: "2", testId: 2 }]) { count } }"#),
          @r###"{"data":{"createManyChild":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateManyChildAndReturn(
              where: {}
              data: { str1: "updated" }
            ) { id str1 }
          }"#),
          @r###"{"data":{"updateManyChildAndReturn":[{"id":1,"str1":"updated"},{"id":2,"str1":"updated"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_1m_child))]
    async fn update_many_1m_non_inline_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Test",
              "action": "updateManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "children": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'children' not found in enclosing type".to_string()));

        Ok(())
    }

    fn schema_m2m_child() -> String {
        let schema = indoc! {
            r#"model Test {
                #id(id, Int, @id)
                str1 String?

                #m2m(children, Child[], id, Int)
              }

              model Child {
                #id(id, Int, @id)

                #m2m(tests, Test[], id, Int)

              }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_m2m_child))]
    async fn update_many_m2m_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Test",
              "action": "updateManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "children": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'children' not found in enclosing type".to_string()));

        runner
            .query_json(serde_json::json!({
              "modelName": "Child",
              "action": "updateManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "tests": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'tests' not found in enclosing type".to_string()));

        Ok(())
    }

    fn schema_self_rel_child() -> String {
        let schema = indoc! {
            r#"model Child {
                  #id(id, Int, @id)

                  teacherId Int?
                  teacher   Child?  @relation("TeacherStudents", fields: [teacherId], references: [id])
                  students  Child[] @relation("TeacherStudents")
                }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_self_rel_child))]
    async fn update_many_self_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Child",
              "action": "updateManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "students": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'students' not found in enclosing type".to_string()));

        Ok(())
    }

    async fn query_number_operation(runner: &Runner, field: &str, op: &str, value: &str) -> TestResult<String> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  updateManyTestModelAndReturn(
                    where: {{}}
                    data: {{ {field}: {{ {op}: {value} }} }}
                  ){{
                    {field}
                  }}
                }}"#
            )
        );

        Ok(res["data"]["updateManyTestModelAndReturn"].to_string())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();

        Ok(())
    }
}

#[test_suite(schema(json_opt), capabilities(AdvancedJsonNullability, UpdateReturning))]
mod json_update_many_and_return {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn update_json_adv(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateManyTestModelAndReturn(where: { id: 1 }, data: { json: "{}" }) { json }}"#),
          @r###"{"data":{"updateManyTestModelAndReturn":[{"json":"{}"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateManyTestModelAndReturn(where: { id: 1 }, data: { json: JsonNull }) { json }}"#),
          @r###"{"data":{"updateManyTestModelAndReturn":[{"json":"null"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateManyTestModelAndReturn(where: { id: 1 }, data: { json: DbNull }) { json }}"#),
          @r###"{"data":{"updateManyTestModelAndReturn":[{"json":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_json_errors(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
                updateManyTestModelAndReturn(where: { id: 1 }, data: { json: AnyNull }) {
                  id
                }
              }"#,
            2009,
            "`AnyNull` is not a valid `NullableJsonNullValueInput`"
        );

        Ok(())
    }
}
