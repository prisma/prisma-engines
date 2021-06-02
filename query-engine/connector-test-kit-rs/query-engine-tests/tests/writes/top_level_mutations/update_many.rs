use query_engine_tests::*;

#[test_suite(schema(schema))]
mod update_many {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

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

    // "An updateMany mutation" should "update the records matching the where clause"
    // TODO(dom): Not working on Mongo (second snapshot)
    // -{"data":{"findManyTestModel":[{"optStr":"str1new","optInt":1,"optFloat":null},{"optStr":"str2","optInt":null,"optFloat":null}]}}
    // +{"data":{"findManyTestModel":[{"optStr":"str1new","optInt":1,"optFloat":0.0},{"optStr":"str2","optInt":null,"optFloat":null}]}}
    #[connector_test(exclude(MongoDb))]
    async fn update_recs_matching_where(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(runner, r#"{ id: 2, optStr: "str2" }"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyTestModel(
              where: { optStr: { equals: "str1" } }
              data: { optStr: { set: "str1new" }, optInt: { set: 1 }, optFloat: { multiply: 2 } }
            ) {
              count
            }
          }"#),
          @r###"{"data":{"updateManyTestModel":{"count":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: { id: asc }) {
              optStr
              optInt
              optFloat
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"optStr":"str1new","optInt":1,"optFloat":null},{"optStr":"str2","optInt":null,"optFloat":null}]}}"###
        );

        Ok(())
    }

    // "An updateMany mutation" should "update the records matching the where clause using shorthands"
    // TODO(dom): Not working on mongo (second snapshot)
    //-{"data":{"findManyTestModel":[{"optStr":"str1new","optInt":null,"optFloat":null},{"optStr":"str2","optInt":null,"optFloat":null}]}}
    //+{"data":{"findManyTestModel":[{"optStr":"str1new","optInt":null,"optFloat":0.0},{"optStr":"str2","optInt":null,"optFloat":null}]}}
    #[connector_test(exclude(MongoDb))]
    async fn update_recs_matching_where_shorthands(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(runner, r#"{ id: 2, optStr: "str2" }"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyTestModel(
              where: { optStr: "str1" }
              data: { optStr: "str1new", optInt: null, optFloat: { multiply: 2 } }
            ) {
              count
            }
          }"#),
          @r###"{"data":{"updateManyTestModel":{"count":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: { id: asc }) {
              optStr
              optInt
              optFloat
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"optStr":"str1new","optInt":null,"optFloat":null},{"optStr":"str2","optInt":null,"optFloat":null}]}}"###
        );

        Ok(())
    }

    // "An updateMany mutation" should "update all items if the where clause is empty"
    // TODO(dom): Not working on Mongo (second snapshot)
    // -{"data":{"findManyTestModel":[{"optStr":"updated","optInt":null,"optFloat":null},{"optStr":"updated","optInt":1,"optFloat":null},{"optStr":"updated","optInt":2,"optFloat":1.55}]}}
    // +{"data":{"findManyTestModel":[{"optStr":"updated","optInt":-1,"optFloat":0.0},{"optStr":"updated","optInt":1,"optFloat":0.0},{"optStr":"updated","optInt":2,"optFloat":1.55}]}}
    #[connector_test(exclude(MongoDb))]
    async fn update_all_items_if_where_empty(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(runner, r#"{ id: 2, optStr: "str2", optInt: 2 }"#).await?;
        create_row(runner, r#"{ id: 3, optStr: "str3", optInt: 3, optFloat: 3.1 }"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyTestModel(
              where: { }
              data: { optStr: { set: "updated" }, optFloat: { divide: 2 }, optInt: { decrement: 1 } }
            ){
              count
            }
          }"#),
          @r###"{"data":{"updateManyTestModel":{"count":3}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel {
              optStr
              optInt
              optFloat
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"optStr":"updated","optInt":null,"optFloat":null},{"optStr":"updated","optInt":1,"optFloat":null},{"optStr":"updated","optInt":2,"optFloat":1.55}]}}"###
        );

        Ok(())
    }

    // "An updateMany mutation" should "correctly apply all number operations for Int"
    // TODO(dom): Not working on Mongo (first snapshot)
    //-{"data":{"findManyTestModel":[{"optInt":null},{"optInt":12},{"optInt":13}]}}
    //+{"data":{"findManyTestModel":[{"optInt":10},{"optInt":12},{"optInt":13}]}}
    #[connector_test(exclude(MongoDb))]
    async fn apply_number_ops_for_int(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(runner, r#"{ id: 2, optStr: "str2", optInt: 2 }"#).await?;
        create_row(runner, r#"{ id: 3, optStr: "str3", optInt: 3, optFloat: 3.1 }"#).await?;

        insta::assert_snapshot!(
          query_number_operation(runner, "optInt", "increment", "10").await?,
          @r###"{"data":{"findManyTestModel":[{"optInt":null},{"optInt":12},{"optInt":13}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optInt", "decrement", "10").await?,
          @r###"{"data":{"findManyTestModel":[{"optInt":null},{"optInt":2},{"optInt":3}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optInt", "multiply", "2").await?,
          @r###"{"data":{"findManyTestModel":[{"optInt":null},{"optInt":4},{"optInt":6}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optInt", "divide", "3").await?,
          @r###"{"data":{"findManyTestModel":[{"optInt":null},{"optInt":1},{"optInt":2}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optInt", "set", "5").await?,
          @r###"{"data":{"findManyTestModel":[{"optInt":5},{"optInt":5},{"optInt":5}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optInt", "set", "null").await?,
          @r###"{"data":{"findManyTestModel":[{"optInt":null},{"optInt":null},{"optInt":null}]}}"###
        );

        Ok(())
    }

    // "An updateMany mutation" should "correctly apply all number operations for Float"
    // TODO(dom): Not working on Mongo (first snapshot)
    //-{"data":{"findManyTestModel":[{"optFloat":null},{"optFloat":3.1},{"optFloat":4.2}]}}
    //+{"data":{"findManyTestModel":[{"optFloat":1.1},{"optFloat":3.1},{"optFloat":4.2}]}}
    #[connector_test(exclude(MongoDb))]
    async fn apply_number_ops_for_float(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, optStr: "str1" }"#).await?;
        create_row(runner, r#"{ id: 2, optStr: "str2", optFloat: 2 }"#).await?;
        create_row(runner, r#"{ id: 3, optStr: "str3", optFloat: 3.1 }"#).await?;

        insta::assert_snapshot!(
          query_number_operation(runner, "optFloat", "increment", "1.1").await?,
          @r###"{"data":{"findManyTestModel":[{"optFloat":null},{"optFloat":3.1},{"optFloat":4.2}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optFloat", "decrement", "1.1").await?,
          @r###"{"data":{"findManyTestModel":[{"optFloat":null},{"optFloat":2.0},{"optFloat":3.1}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optFloat", "multiply", "5.5").await?,
          @r###"{"data":{"findManyTestModel":[{"optFloat":null},{"optFloat":11.0},{"optFloat":17.05}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optFloat", "divide", "2").await?,
          @r###"{"data":{"findManyTestModel":[{"optFloat":null},{"optFloat":5.5},{"optFloat":8.525}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optFloat", "set", "5").await?,
          @r###"{"data":{"findManyTestModel":[{"optFloat":5.0},{"optFloat":5.0},{"optFloat":5.0}]}}"###
        );

        insta::assert_snapshot!(
          query_number_operation(runner, "optFloat", "set", "null").await?,
          @r###"{"data":{"findManyTestModel":[{"optFloat":null},{"optFloat":null},{"optFloat":null}]}}"###
        );

        Ok(())
    }

    async fn query_number_operation(runner: &Runner, field: &str, op: &str, value: &str) -> TestResult<String> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
          updateManyTestModel(
            where: {{}}
            data: {{ {}: {{ {}: {} }} }}
          ){{
            count
          }}
        }}"#,
                field, op, value
            )
        );
        let count = &res["data"]["updateManyTestModel"]["count"];

        assert_eq!(count, 3);

        let res = run_query!(runner, format!(r#"{{ findManyTestModel {{ {} }} }}"#, field));

        Ok(res)
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
