use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(ScalarLists, Json, JsonLists))]
mod json {
    use indoc::indoc;
    use query_engine_tests::{run_query, Runner};

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

        if runner.protocol().is_graphql() {
            insta::assert_snapshot!(
              run_query!(&runner, r#"mutation {
              updateOneScalarModel(where: { id: 1 }, data: {
                jsons:  { push: ["[]", "{}"] }
              }) {
                jsons
              }
            }"#),
              @r###"{"data":{"updateOneScalarModel":{"jsons":["{\"a\":\"b\"}","{}","2","[]","{}"]}}}"###
            );
        } else {
          // The request transformation doesn't work well with those queries. ["[]", "{}"] ends up deserialized as [[], {}]
            let query = r#"{
            "modelName": "ScalarModel",
            "action": "updateOne",
            "query": {
              "arguments": {
                "where": { "id": 1 },
                "data": { "jsons": { "push": ["[]", "{}"] } }
              },
              "selection": { "jsons": true }
            }
          }"#;

            let res = runner.query_json(query).await?;
            res.assert_success();

            insta::assert_snapshot!(
              res.to_string(),
              @r###"{"data":{"updateOneScalarModel":{"jsons":[{"$type":"Json","value":"{\"a\":\"b\"}"},{"$type":"Json","value":"{}"},{"$type":"Json","value":"2"},{"$type":"Json","value":"[\"[]\",\"{}\"]"}]}}}"###
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

        if runner.protocol().is_graphql() {
            insta::assert_snapshot!(
              run_query!(
                &runner,
                r#"mutation {
                  updateOneScalarModel(where: { id: 2 }, data: {
                    jsons:  { push: ["1", "2"] }
                  }) {
                    jsons
                  }
                }"#
              ),
              @r###"{"data":{"updateOneScalarModel":{"jsons":["1","2"]}}}"###
            );
        } else {
            // The request transformation doesn't work well with those queries. ["1", "2"] ends up deserialized as [1, 2]
            let query = r#"{
              "modelName": "ScalarModel",
              "action": "updateOne",
              "query": {
                "arguments": {
                  "where": { "id": 2 },
                  "data": { "jsons": { "push": ["1", "2"] } }
                },
                "selection": { "jsons": true }
              }
            }"#;

            let res = runner.query_json(query).await?;
            res.assert_success();

            insta::assert_snapshot!(
              res.to_string(),
              @r###"{"data":{"updateOneScalarModel":{"jsons":[{"$type":"Json","value":"[\"1\",\"2\"]"}]}}}"###
            );
        };

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
