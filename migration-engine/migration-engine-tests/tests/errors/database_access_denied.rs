use migration_engine_tests::{multi_engine_test_api::*, TestResult};
use quaint::prelude::*;
use serde_json::json;
use test_macros::test_connector;
use url::Url;

#[test_connector(tags(Mysql57), exclude(Vitess))]
async fn database_access_denied_must_return_a_proper_error_in_rpc(api: &TestApi) -> TestResult {
    // let db_name = "dbaccessdeniedinrpc";
    // let url: Url = mysql_5_7_url(db_name).0.parse().unwrap();
    // let conn = create_mysql_database(&url).await.unwrap();

    api.admin_conn()
        .execute_raw("DROP USER IF EXISTS jeanyves", &[])
        .await
        .unwrap();
    api.admin_conn()
        .execute_raw("CREATE USER jeanyves IDENTIFIED BY '1234'", &[])
        .await
        .unwrap();

    let mut url: Url = api.connection_string().parse()?;
    url.set_username("jeanyves").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("/access_denied_test");

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url,
    );

    let error = migration_core::api::RpcApi::new(&dm).await.map(|_| ()).unwrap_err();
    let json_error = serde_json::to_value(&error.to_user_facing()).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": "User `jeanyves` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanyves",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);

    Ok(())
}
