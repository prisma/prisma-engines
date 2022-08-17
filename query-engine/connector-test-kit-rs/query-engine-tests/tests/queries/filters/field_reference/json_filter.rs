use query_engine_tests::*;

#[test_suite(capabilities(JsonFiltering), exclude(MySql(5.6)))]
mod json_filter {
    use query_engine_tests::run_query;

    pub fn schema() -> String {
        let schema = indoc! {
          "model TestModel {
            #id(id, Int, @id)
            json     Json?
            json2    Json?
          }"
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema))]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: { _ref: "json" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: { _ref: "json2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { not: { _ref: "json2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema), capabilities(JsonFilteringAlphanumericFieldRef))]
    async fn numeric_comparison_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, gt: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, gt: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, gte: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, gte: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, lt: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, lt: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, lte: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, lte: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema))]
    async fn string_comparison_filters(runner: Runner) -> TestResult<()> {
        test_string_data(&runner).await?;

        // contains
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, string_contains: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, string_contains: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );

        // not contains
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, string_contains: {{ _ref: "json" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, string_contains: {{ _ref: "json2" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // startsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, string_starts_with: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, string_starts_with: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // not startsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, string_starts_with: {{ _ref: "json" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, string_starts_with: {{ _ref: "json2" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // endsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, string_ends_with: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, string_ends_with: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3}]}}"###
        );

        // not endsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, string_ends_with: {{ _ref: "json" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, string_ends_with: {{ _ref: "json2" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema))]
    async fn array_comparison_filters(runner: Runner) -> TestResult<()> {
        test_array_data(&runner).await?;

        // contains
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, array_contains: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, array_contains: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // not contains
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, array_contains: {{ _ref: "json" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, array_contains: {{ _ref: "json2" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // startsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, array_starts_with: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, array_starts_with: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not startsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, array_starts_with: {{ _ref: "json" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, array_starts_with: {{ _ref: "json2" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );

        // endsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, array_ends_with: {{ _ref: "json" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"json: {{ {}, array_ends_with: {{ _ref: "json2" }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // not endsWith
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, array_ends_with: {{ _ref: "json" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, jsonq(format!(r#"NOT: {{ json: {{ {}, array_ends_with: {{ _ref: "json2" }} }} }}"#, json_path(&runner)))),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3}]}}"###
        );

        Ok(())
    }

    fn schema_list() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              json       Json?
              json_list  Json[]
              json_list2 Json[]
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_list), capabilities(JsonFiltering, ScalarLists))]
    async fn scalar_list_filters(runner: Runner) -> TestResult<()> {
        test_data_list(&runner).await?;

        // has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json_list: { has: { _ref: "json" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { json_list: { has: { _ref: "json" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json_list: { hasSome: { _ref: "json_list" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json_list: { hasSome: { _ref: "json_list2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // not hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { json_list: { hasSome: { _ref: "json_list" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { json_list: { hasSome: { _ref: "json_list2" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json_list: { hasEvery: { _ref: "json_list" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json_list: { hasEvery: { _ref: "json_list2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { json_list: { hasEvery: { _ref: "json_list" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { json_list: { hasEvery: { _ref: "json_list2" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    pub async fn test_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 1,
              json: "{\"a\":{\"b\":\"c\"}}",
              json2: "{\"a\":{\"b\":\"c\"}}",
            }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 2,
              json: "{\"a\":{\"b\":\"a\"}}",
              json2: "\"b\""
            }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
                id: 3,
                json: "{\"a\":{\"b\":2}}",
                json2: "1",
            }) { id }}"#
        );

        run_query!(runner, r#"mutation { createOneTestModel(data: { id: 4 }) { id }}"#);

        Ok(())
    }

    pub async fn test_string_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
            id: 1,
            json: "{\"a\":{\"b\":\"abba\"}}",
            json2: "\"abba\"",
          }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 2,
              json: "{\"a\":{\"b\":\"abba\"}}",
              json2: "\"ab\"",
            }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 3,
              json: "{\"a\":{\"b\":\"abba\"}}",
              json2: "\"ba\"",
            }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 4,
              json: "{\"a\":{\"b\":\"1\"}}",
              json2: "1",
            }) { id }}"#
        );

        run_query!(runner, r#"mutation { createOneTestModel(data: { id: 5 }) { id }}"#);

        Ok(())
    }

    pub async fn test_array_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
            id: 1,
            json: "{\"a\":{\"b\":[\"bob\", \"alice\"]}}",
            json2: "\"bob\"",
          }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 2,
              json: "{\"a\":{\"b\":[\"bob\", \"alice\"]}}",
              json2: "\"alice\"",
            }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 3,
              json: "{\"a\":{\"b\":[\"bob\", \"alice\"]}}",
              json2: "\"john\"",
            }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 4,
              json: "{\"a\":{\"b\": \"alice\"}}",
              json2: "\"alice\"",
            }) { id }}"#
        );

        run_query!(runner, r#"mutation { createOneTestModel(data: { id: 5 }) { id }}"#);

        Ok(())
    }

    pub async fn test_data_list(runner: &Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"
              mutation { createOneTestModel(data: {
                  id: 1,
                  json: "{\"a\":1}",
                  json_list: ["{\"a\":1}", "{\"a\":1}"],
                  json_list2: ["{\"a\":1}", "{\"a\":1}"],
              }) { id }}
            "#
        );

        run_query!(
            &runner,
            r#"
              mutation { createOneTestModel(data: {
                  id: 2,
                  json: "{\"a\":4}",
                  json_list: ["{\"a\":1}", "{\"a\":2}"],
                  json_list2: ["{\"a\":2}", "{\"a\":3}"],
              }) { id }}
            "#
        );

        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#);

        Ok(())
    }

    fn jsonq(filter: String) -> String {
        format!(r#"query {{ findManyTestModel(where: {{ {} }} ) {{ id }} }}"#, filter)
    }

    fn json_path(runner: &Runner) -> &'static str {
        match runner.connector_version() {
            ConnectorVersion::Postgres(_) | ConnectorVersion::CockroachDb => r#"path: ["a", "b"]"#,
            ConnectorVersion::MySql(_) => r#"path: "$.a.b""#,
            x => unreachable!("JSON filtering is not supported on {:?}", x),
        }
    }
}
