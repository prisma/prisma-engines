use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(ScalarLists, Json, JsonLists))]
mod json {
    use indoc::indoc;
    use query_engine_tests::{Runner, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              jsons Json[]
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(exclude(CockroachDb))]
    async fn behave_like_regular_val_for_create_and_update(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: 1,
              jsons: { set: ["{ \"a\": [\"b\"] }", "3"] }
            }) {
              jsons
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"jsons":["{\"a\":[\"b\"]}","3"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
                jsons: { set: ["{ \"a\": \"b\" }", "{}"] }
            }) {
              jsons
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              jsons:  { push: "2" }
            }) {
              jsons
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}","2"]}}}"###
        );

        match_connector_result!(
          &runner,
          r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              jsons:  { push: "[[], {}]" }
            }) {
              jsons
            }
          }"#,
          // MongoDB behaves differently. This is a bug.
          // https://github.com/prisma/prisma/issues/18019
          MongoDb(_) => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}","2","[]","{}"]}}}"#],
          _ => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}","2","[[],{}]"]}}}"#]
        );

        // TODO: This specific query currently cannot be sent from the JS client.
        // The client _always_ sends an array as plain json and never as an array of json.
        // We're temporarily ignoring it for the JSON protocol because we can't differentiate a list of json values from a json array.
        // Similarly, this does not currently work with driver adapters.
        // https://github.com/prisma/prisma/issues/18019
        if runner.protocol().is_graphql() && !runner.is_external_executor() {
            match_connector_result!(
              &runner,
              r#"mutation {
                updateOneScalarModel(where: { id: 1 }, data: {
                  jsons:  { push: ["[]", "{}"] }
                }) {
                  jsons
                }
              }"#,
              MongoDb(_) => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}","2","[]","{}","[]","{}"]}}}"#],
              _ => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}","2","[[],{}]","[]","{}"]}}}"#]
            );
        }

        Ok(())
    }

    // "A Create Mutation" should "create and return items with list values with shorthand notation"
    #[connector_test]
    async fn create_mut_work_with_list_vals(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: 1
              jsons: ["{ \"a\": \"b\" }", "{}"]
            }) {
              jsons
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}"]}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return items with empty list values"
    #[connector_test]
    async fn create_mut_return_items_with_empty_lists(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: 1
              jsons: []
            }) {
              jsons
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"jsons":[]}}}"###
        );

        Ok(())
    }

    // "An Update Mutation that pushes to some empty scalar lists" should "work"
    // Skipped for CockroachDB as enum array concatenation is not supported (https://github.com/cockroachdb/cockroach/issues/71388).
    #[connector_test(exclude(CockroachDb))]
    async fn update_mut_push_empty_scalar_list(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              jsons:  { push: "2" }
            }) {
              jsons
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"jsons":["2"]}}}"###
        );

        match_connector_result!(
          &runner,
          r#"mutation {
            updateOneScalarModel(where: { id: 2 }, data: {
              jsons:  { push: "[\"1\", \"2\"]" }
            }) {
              jsons
            }
          }"#,
          // MongoDB behaves differently. This is a bug.
          // https://github.com/prisma/prisma/issues/18019
          MongoDb(_) => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["\"1\"","\"2\""]}}}"#],
          _ => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["[\"1\",\"2\"]"]}}}"#]
        );

        // TODO: This specific query currently cannot be sent from the JS client.
        // The client _always_ sends an array as plain json and never as an array of json.
        // We're temporarily ignoring it for the JSON protocol because we can't differentiate a list of json values from a json array.
        // Similarly, this does not currently work with driver adapters.
        // https://github.com/prisma/prisma/issues/18019
        if runner.protocol().is_graphql() && !runner.is_external_executor() {
            match_connector_result!(
              &runner,
              r#"mutation {
                updateOneScalarModel(where: { id: 2 }, data: {
                  jsons:  { push: ["1", "2"] }
                }) {
                  jsons
                }
              }"#,
              MongoDb(_) => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["\"1\"","\"2\"","1","2"]}}}"#],
              _ => vec![r#"{"data":{"updateOneScalarModel":{"jsons":["[\"1\",\"2\"]","1","2"]}}}"#]
            );
        }

        Ok(())
    }

    #[connector_test]
    async fn push_json_protocol(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;

        if runner.protocol().is_json() {
            let query = r#"{
            "modelName": "ScalarModel",
            "action": "updateOne",
            "query": {
              "arguments": {
                "where": { "id": 1 },
                "data": {
                  "jsons":  { "push": [{ "$type": "Json", "value": "1" }, { "$type": "Json", "value": "true" }] }
                }
              },
              "selection": { "$scalars": true }
            }
          }"#;

            let res = runner.query_json(query).await?;

            insta::assert_snapshot!(
              res.to_string(),
              @r###"{"data":{"updateOneScalarModel":{"id":1,"jsons":[{"$type":"Json","value":"1"},{"$type":"Json","value":"true"}]}}}"###
            );
        }

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneScalarModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
