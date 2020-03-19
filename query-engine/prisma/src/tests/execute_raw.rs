use super::test_api::*;
use indoc::indoc;
use quaint::ast::*;
use quaint::connector::ConnectionInfo;
use serde_json::json;
use test_macros::*;

static TODO: &str = indoc! {"
    model Todo {
        id String @id @default(cuid())
        title String
    }
"};

fn execute_raw(query: &str, params: Vec<ParameterizedValue>) -> String {
    let params: Vec<serde_json::Value> = params.into_iter().map(serde_json::Value::from).collect();
    let params = serde_json::to_string(&params).unwrap();

    format!(
        r#"mutation {{ executeRaw(query: "{}", parameters: "{}") }}"#,
        query.replace("\"", "\\\""),
        params.replace("\"", "\\\"")
    )
}

#[test_each_connector]
async fn select_1(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let query = indoc! {r#"
        mutation {
            executeRaw(
                query: "SELECT 1"
            )
        }
    "#};

    let column_name = match api.connection_info() {
        ConnectionInfo::Postgres(_) => "?column?",
        _ => "1",
    };

    assert_eq!(
        json!({
            "data": {
                "executeRaw": [{column_name: 1}]
            }
        }),
        query_engine.request(query).await
    );

    Ok(())
}

#[test_each_connector]
async fn parameterized_queries(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let query = match api.connection_info() {
        ConnectionInfo::Postgres(_) => {
            indoc! {r#"
                mutation {
                    executeRaw(
                        query: "SELECT ($1)::text",
                        parameters: "[\"foo\"]"
                    )
                }
            "#}
        }
        _ => {
            indoc! {r#"
                mutation {
                    executeRaw(
                        query: "SELECT ?",
                        parameters: "[\"foo\"]"
                    )
                }
            "#}
        }
    };

    let column_name = match api.connection_info() {
        ConnectionInfo::Postgres(_) => "text",
        _ => "?",
    };

    assert_eq!(
        json!({
            "data": {
                "executeRaw": [{column_name: "foo"}]
            }
        }),
        query_engine.request(query).await
    );

    Ok(())
}

#[test_each_connector]
async fn querying_model_tables(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let mutation = indoc! {r#"
        mutation {
            createOneTodo(data: { title: "title1" }) { id }
        }
    "#};

    let res = query_engine.request(mutation).await;
    let id = res["data"]["createOneTodo"]["id"].as_str().unwrap();

    let (query, _) = api.to_sql_string(Select::from_table("Todo").value(asterisk()));

    assert_eq!(
        json!({
            "data": {
                "executeRaw": [
                    {"id": id, "title": "title1"}
                ]
            }
        }),
        query_engine.request(execute_raw(&query, vec![])).await
    );

    Ok(())
}

#[test_each_connector]
async fn inserting_into_model_table(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let insert = Insert::multi_into("Todo", vec!["id", "title"])
        .values(vec!["id1", "title1"])
        .values(vec!["id2", "title2"]);

    let (query, params) = api.to_sql_string(insert);

    assert_eq!(
        json!({
            "data": {
                "executeRaw": 2
            }
        }),
        query_engine.request(execute_raw(&query, params)).await,
    );

    let (query, _) = api.to_sql_string(Select::from_table("Todo").value(asterisk()));

    assert_eq!(
        json!({
            "data": {
                "executeRaw": [
                    {"id": "id1", "title": "title1"},
                    {"id": "id2", "title": "title2"}
                ]
            }
        }),
        query_engine.request(execute_raw(&query, vec![])).await
    );

    Ok(())
}

#[test_each_connector]
async fn querying_model_tables_with_alias(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let mutation = indoc! {r#"
        mutation {
            createOneTodo(data: { title: "title1" }) { id }
        }
    "#};

    query_engine.request(mutation).await;

    let (query, params) = api.to_sql_string(
        Select::from_table("Todo").column(Column::from("title").alias("aliasedTitle")),
    );

    assert_eq!(
        json!({
            "data": {
                "executeRaw": [{"aliasedTitle": "title1"}]
            }
        }),
        query_engine.request(execute_raw(&query, params)).await,
    );

    Ok(())
}

#[test_each_connector]
async fn querying_the_same_column_name_twice_with_aliasing(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let mutation = indoc! {r#"
        mutation {
            createOneTodo(data: { title: "title1" }) { id }
        }
    "#};

    query_engine.request(mutation).await;

    let select = Select::from_table("Todo")
        .column(Column::from("title").alias("ALIASEDTITLE"))
        .column("title");

    let (query, params) = api.to_sql_string(select);

    assert_eq!(
        json!({
            "data": {
                "executeRaw": [{"ALIASEDTITLE": "title1", "title": "title1"}]
            }
        }),
        query_engine.request(execute_raw(&query, params)).await,
    );

    Ok(())
}

#[test_one_connector(connector = "postgres")]
async fn arrays(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let query =
        "SELECT ARRAY_AGG(columnInfos.attname) AS postgres_array FROM pg_attribute columnInfos";
    let result = query_engine.request(execute_raw(query, vec![])).await;
    let array = result["data"]["executeRaw"][0]["postgres_array"]
        .as_array()
        .unwrap();

    for val in array.into_iter() {
        assert!(val.is_string());
    }

    Ok(())
}

#[test_each_connector]
async fn syntactic_errors_bubbling_through_to_the_user(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;
    let result = query_engine
        .request(execute_raw("SELECT * FROM ", vec![]))
        .await;
    let error_code = result["errors"][0]["user_facing_error"]["meta"]["code"].as_str();

    match api.connection_info() {
        ConnectionInfo::Postgres(..) => assert_eq!(Some("42601"), error_code),
        ConnectionInfo::Mysql(..) => assert_eq!(Some("1064"), error_code),
        ConnectionInfo::Sqlite { .. } => assert_eq!(Some("1"), error_code),
    }

    Ok(())
}

#[test_each_connector]
async fn other_errors_bubbling_through_to_the_user(api: &TestApi) -> anyhow::Result<()> {
    let query_engine = api.create_engine(&TODO).await?;

    let mutation = indoc! {r#"
        mutation {
            createOneTodo(data: { title: "title1" }) { id }
        }
    "#};

    let result = query_engine.request(mutation).await;
    let id = result["data"]["createOneTodo"]["id"].as_str().unwrap();

    let insert = Insert::single_into("Todo")
        .value("id", id)
        .value("title", "irrelevant");
    let (query, params) = api.to_sql_string(insert);

    let result = query_engine.request(execute_raw(&query, params)).await;
    let error_code = result["errors"][0]["user_facing_error"]["meta"]["code"].as_str();

    match api.connection_info() {
        ConnectionInfo::Postgres(..) => assert_eq!(Some("23505"), error_code),
        ConnectionInfo::Mysql(..) => assert_eq!(Some("1062"), error_code),
        ConnectionInfo::Sqlite { .. } => assert_eq!(Some("1555"), error_code),
    }

    Ok(())
}
