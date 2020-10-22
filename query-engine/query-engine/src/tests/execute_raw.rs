use super::test_api::*;
use chrono::{DateTime, Utc};
use indoc::indoc;
use quaint::ast::*;
use quaint::connector::{ConnectionInfo, SqlFamily};
use serde_json::json;
use test_macros::test_each_connector_mssql as test_each_connector;

static TODO: &str = indoc! {"
    model Todo {
        id String @id @default(cuid())
        title String
        dt DateTime?
    }
"};

fn execute_raw(query: &str, params: Vec<Value>) -> String {
    let params: Vec<serde_json::Value> = params
        .into_iter()
        .map(|v| match v {
            Value::DateTime(Some(dt)) => json!({
                "prisma__type": "date",
                "prisma__value": dt.to_rfc3339(),
            }),
            _ => serde_json::Value::from(v),
        })
        .collect();

    let params = serde_json::to_string(&params).unwrap();

    format!(
        r#"mutation {{ executeRaw(query: "{}", parameters: "{}") }}"#,
        query.replace("\"", "\\\""),
        params.replace("\"", "\\\"")
    )
}

fn query_raw(query: &str, params: Vec<Value>) -> String {
    let params: Vec<serde_json::Value> = params.into_iter().map(serde_json::Value::from).collect();
    let params = serde_json::to_string(&params).unwrap();

    format!(
        r#"mutation {{ queryRaw(query: "{}", parameters: "{}") }}"#,
        query.replace("\"", "\\\""),
        params.replace("\"", "\\\"")
    )
}

#[test_each_connector]
async fn select_1(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let query = indoc! {r#"
        mutation {
            queryRaw(
                query: "SELECT 1 AS result"
            )
        }
    "#};

    assert_eq!(
        json!({
            "data": {
                "queryRaw": [{"result": 1}]
            }
        }),
        query_engine.request(query).await
    );

    Ok(())
}

#[test_each_connector]
async fn parameterized_queries(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let query = match api.connection_info() {
        ConnectionInfo::Postgres(_) => {
            indoc! {r#"
                mutation {
                    queryRaw(
                        query: "SELECT ($1)::text AS result",
                        parameters: "[\"foo\"]"
                    )
                }
            "#}
        }
        ConnectionInfo::Mssql(_) => {
            indoc! {r#"
                mutation {
                    queryRaw(
                        query: "SELECT @P1 AS result",
                        parameters: "[\"foo\"]"
                    )
                }
            "#}
        }
        _ => {
            indoc! {r#"
                mutation {
                    queryRaw(
                        query: "SELECT ? AS result",
                        parameters: "[\"foo\"]"
                    )
                }
            "#}
        }
    };

    assert_eq!(
        json!({
            "data": {
                "queryRaw": [{"result": "foo"}]
            }
        }),
        query_engine.request(query).await
    );

    Ok(())
}

#[test_each_connector]
async fn querying_model_tables(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let mutation = indoc! {r#"
        mutation {
            createOneTodo(data: { title: "title1" }) { id }
        }
    "#};

    let res = query_engine.request(mutation).await;
    let id = res["data"]["createOneTodo"]["id"].as_str().unwrap();

    let (query, _) = api.to_sql_string(Select::from_table("Todo").value(asterisk()))?;

    assert_eq!(
        json!({
            "data": {
                "queryRaw": [
                    {"id": id, "title": "title1", "dt": serde_json::Value::Null}
                ]
            }
        }),
        query_engine.request(query_raw(&query, vec![])).await
    );

    Ok(())
}

#[test_each_connector]
async fn inserting_into_model_table(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let dt = DateTime::parse_from_rfc3339("1996-12-19T16:39:57+00:00")?;
    let dt: DateTime<Utc> = dt.into();

    let insert = Insert::multi_into("Todo", &["id", "title", "dt"])
        .values(("id1", "title1", dt))
        .values(("id2", "title2", dt));

    let (query, params) = api.to_sql_string(insert)?;

    assert_eq!(
        json!({
            "data": {
                "executeRaw": 2
            }
        }),
        query_engine.request(execute_raw(&query, params)).await,
    );

    let (query, _) = api.to_sql_string(Select::from_table("Todo").value(asterisk()))?;

    match api.connection_info().sql_family() {
        SqlFamily::Sqlite => {
            assert_eq!(
                json!({
                    "data": {
                        "queryRaw": [
                            {"id": "id1", "title": "title1", "dt": "1996-12-19T16:39:57+00:00"},
                            {"id": "id2", "title": "title2", "dt": "1996-12-19T16:39:57+00:00"}
                        ]
                    }
                }),
                query_engine.request(query_raw(&query, vec![])).await
            );
        }
        _ => {
            assert_eq!(
                json!({
                    "data": {
                        "queryRaw": [
                            {"id": "id1", "title": "title1", "dt": "1996-12-19T16:39:57+00:00"},
                            {"id": "id2", "title": "title2", "dt": "1996-12-19T16:39:57+00:00"}
                        ]
                    }
                }),
                query_engine.request(query_raw(&query, vec![])).await
            );
        }
    }

    Ok(())
}

#[test_each_connector]
async fn querying_model_tables_with_alias(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let mutation = indoc! {r#"
        mutation {
            createOneTodo(data: { title: "title1" }) { id }
        }
    "#};

    query_engine.request(mutation).await;

    let (query, params) =
        api.to_sql_string(Select::from_table("Todo").column(Column::from("title").alias("aliasedTitle")))?;

    assert_eq!(
        json!({
            "data": {
                "queryRaw": [{"aliasedTitle": "title1"}]
            }
        }),
        query_engine.request(query_raw(&query, params)).await,
    );

    Ok(())
}

#[test_each_connector]
async fn querying_the_same_column_name_twice_with_aliasing(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
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

    let (query, params) = api.to_sql_string(select)?;

    assert_eq!(
        json!({
            "data": {
                "queryRaw": [{"ALIASEDTITLE": "title1", "title": "title1"}]
            }
        }),
        query_engine.request(query_raw(&query, params)).await,
    );

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn arrays(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let query = "SELECT ARRAY_AGG(columnInfos.attname) AS postgres_array FROM pg_attribute columnInfos";
    let result = query_engine.request(query_raw(query, vec![])).await;
    let array = result["data"]["queryRaw"][0]["postgres_array"].as_array().unwrap();

    for val in array.iter() {
        assert!(val.is_string());
    }

    Ok(())
}

#[test_each_connector]
async fn syntactic_errors_bubbling_through_to_the_user(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;
    let result = query_engine.request(query_raw("SELECT * FROM ", vec![])).await;
    let error_code = result["errors"][0]["user_facing_error"]["meta"]["code"].as_str();

    match api.connection_info() {
        ConnectionInfo::Postgres(..) => assert_eq!(Some("42601"), error_code),
        ConnectionInfo::Mysql(..) => assert_eq!(Some("1064"), error_code),
        ConnectionInfo::Sqlite { .. } | ConnectionInfo::InMemorySqlite { .. } => assert_eq!(Some("1"), error_code),
        ConnectionInfo::Mssql(..) => assert_eq!(Some("102"), error_code),
        ConnectionInfo::InMemorySqlite { .. } => todo!("Not yet"),
    }

    Ok(())
}

#[test_each_connector]
async fn other_errors_bubbling_through_to_the_user(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let mutation = indoc! {r#"
        mutation {
            createOneTodo(data: { title: "title1" }) { id }
        }
    "#};

    let result = query_engine.request(mutation).await;
    let id = result["data"]["createOneTodo"]["id"].as_str().unwrap();

    let insert = Insert::single_into("Todo").value("id", id).value("title", "irrelevant");
    let (query, params) = api.to_sql_string(insert)?;

    let result = query_engine.request(execute_raw(&query, params)).await;
    let error_code = result["errors"][0]["user_facing_error"]["meta"]["code"].as_str();

    match api.connection_info() {
        ConnectionInfo::Postgres(..) => assert_eq!(Some("23505"), error_code),
        ConnectionInfo::Mysql(..) => assert_eq!(Some("1062"), error_code),
        ConnectionInfo::Sqlite { .. } | ConnectionInfo::InMemorySqlite { .. } => assert_eq!(Some("1555"), error_code),
        ConnectionInfo::Mssql { .. } => assert_eq!(Some("2627"), error_code),
        ConnectionInfo::InMemorySqlite { .. } => todo!("Not yet"),
    }

    Ok(())
}

#[test_each_connector]
async fn parameter_escaping(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&TODO).await?;

    let query = match api.connection_info() {
        ConnectionInfo::Postgres(_) => {
            indoc! {r#"
                mutation {
                    queryRaw(
                        query: "SELECT ($1)::text AS result",
                        parameters: "[\"\\\"name\\\"\"]"
                    )
                }
            "#}
        }
        ConnectionInfo::Mssql(_) => {
            indoc! {r#"
                mutation {
                    queryRaw(
                        query: "SELECT @P1 AS result",
                        parameters: "[\"\\\"name\\\"\"]"
                    )
                }
            "#}
        }
        _ => {
            indoc! {r#"
                mutation {
                    queryRaw(
                        query: "SELECT ? AS result",
                        parameters: "[\"\\\"name\\\"\"]"
                    )
                }
            "#}
        }
    };

    assert_eq!(
        json!({
            "data": {
                "queryRaw": [{"result": "\"name\""}]
            }
        }),
        query_engine.request(query).await
    );

    Ok(())
}
