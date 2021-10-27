use indoc::indoc;
use introspection_engine_tests::{test_api::*, TestResult};
use sql_introspection_connector::SqlIntrospectionConnector;
use test_macros::test_connector;
use url::Url;

#[test_connector(tags(Mysql))]
async fn empty_result_without_select_rights(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (
            id INT PRIMARY KEY
        );

        CREATE USER IF NOT EXISTS test_user IDENTIFIED BY 'test';
        GRANT CREATE ON empty_result_without_select_rights.* to test_user;
    "#};

    api.raw_cmd(setup).await;

    let expected_for_root = expect![[r#"
        model A {
          id Int @id
        }
    "#]];

    expected_for_root.assert_eq(&api.introspect_dml().await?);

    let mut url = Url::parse(api.connection_string()).unwrap();
    url.set_username("test_user").unwrap();
    url.set_password(Some("test")).unwrap();

    let test_api = SqlIntrospectionConnector::new(&url.to_string(), BitFlags::all())
        .await
        .unwrap();

    let schema = test_api.describe().await?;
    assert!(schema.tables.is_empty());

    Ok(())
}
