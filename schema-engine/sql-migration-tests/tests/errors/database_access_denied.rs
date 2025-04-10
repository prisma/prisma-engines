use super::error_tests::connection_error;
use expect_test::expect;
use sql_migration_tests::multi_engine_test_api::*;
use test_macros::test_connector;
use url::Url;

#[test_connector(tags(Mysql57), exclude(Vitess))]
fn database_access_denied_must_return_a_proper_error_in_rpc(api: TestApi) {
    api.raw_cmd("DROP USER IF EXISTS jeanyves");
    api.raw_cmd("CREATE USER jeanyves IDENTIFIED BY '1234'");

    let mut url: Url = api.connection_string().parse().unwrap();
    url.set_username("jeanyves").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("/access_denied_test");

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{url}"
            }}
        "#,
    );

    let error = tok(connection_error(dm));
    let json_error = serde_json::to_string_pretty(&error.to_user_facing()).unwrap();

    let expected = expect![[r#"
        {
          "is_panic": false,
          "message": "User was denied access on the database `access_denied_test`",
          "meta": {
            "database_name": "access_denied_test"
          },
          "error_code": "P1010"
        }"#]];
    expected.assert_eq(&json_error);
}
