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
          @r###"[{"optFloat":null},{"optFloat":2.0},{"optFloat":3.1}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "multiply", "5.5").await?,
          @r###"[{"optFloat":null},{"optFloat":11.0},{"optFloat":17.05}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "divide", "2").await?,
          @r###"[{"optFloat":null},{"optFloat":5.5},{"optFloat":8.525}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "set", "5").await?,
          @r###"[{"optFloat":5.0},{"optFloat":5.0},{"optFloat":5.0}]"###
        );

        insta::assert_snapshot!(
          query_number_operation(&runner, "optFloat", "set", "null").await?,
          @r###"[{"optFloat":null},{"optFloat":null},{"optFloat":null}]"###
        );

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

#[test_suite(schema(json_opt), exclude(MySql(5.6)), capabilities(Json))]
mod json_update_many_and_return {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test(capabilities(AdvancedJsonNullability))]
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

    #[connector_test(capabilities(AdvancedJsonNullability))]
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
