use crate::helpers::*;
use query_engine_tests::*;

const JSON_QUERY: &str = r#"
{
    "action": "findMany",
    "modelName": "Person",
    "query": {
        "arguments": {
        },
        "selection": {
            "$scalars": true
        }
    }
}
"#;

const GRAPHQL_QUERY: &str = r#"
{
  "operationName": null,
  "variables": {},
  "query": "{\n  findManyPerson {\n    id\n  }\n}\n"
}
"#;

#[test_suite(schema(schema))]
mod mismatched {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Person {
              #id(id, Int, @id)
             }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn json_query_over_json_protocol_engine(r: Runner) -> TestResult<()> {
        let (mut qe_cmd, url) = query_engine_cmd(r.prisma_dml());
        qe_cmd.env("PRISMA_ENGINE_PROTOCOL", "json");

        with_child_process(&mut qe_cmd, async move {
            let client = reqwest::Client::new();
            let res = client.post(url).body(JSON_QUERY).send().await.unwrap();
            insta::assert_snapshot!(res.text().await.unwrap(), @r###"{"data":{"findManyPerson":[]}}"###);
        })
        .await
    }

    #[connector_test]
    async fn graphql_query_over_json_protocol_engine(r: Runner) -> TestResult<()> {
        let (mut qe_cmd, url) = query_engine_cmd(r.prisma_dml());
        qe_cmd.env("PRISMA_ENGINE_PROTOCOL", "json");

        with_child_process(&mut qe_cmd, async move {
            let client = reqwest::Client::new();
            let res = client
                .post(url)
                .body(GRAPHQL_QUERY)
                .send()
                .await
                .unwrap();

            assert_eq!(res.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
            insta::assert_snapshot!(res.text().await.unwrap(), @r###"{"is_panic":false,"message":"Error parsing Json query. Ensure that engine protocol of the client and the engine matches. data did not match any variant of untagged enum JsonBody","backtrace":null}"###);
        })
        .await
    }

    #[connector_test]
    async fn json_query_over_graphql_protocol_engine(r: Runner) -> TestResult<()> {
        let (mut qe_cmd, url) = query_engine_cmd(r.prisma_dml());
        qe_cmd.env("PRISMA_ENGINE_PROTOCOL", "graphql");

        with_child_process(&mut qe_cmd, async move {
            let client = reqwest::Client::new();
            let res = client
                .post(url)
                .body(JSON_QUERY)
                .send()
                .await
                .unwrap();

            assert_eq!(res.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
            insta::assert_snapshot!(res.text().await.unwrap(), @r###"{"is_panic":false,"message":"Error parsing Graphql query. Ensure that engine protocol of the client and the engine matches. data did not match any variant of untagged enum GraphqlBody","backtrace":null}"###);
        })
        .await
    }
}
