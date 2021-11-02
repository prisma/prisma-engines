use crate::test_api::*;
use datamodel::Datamodel;
use introspection_connector::{IntrospectionConnector, IntrospectionContext};
use mongodb_introspection_connector::MongoDbIntrospectionConnector;

#[test]
fn connection_string_problems_give_a_nice_error() {
    let conn_str = "mongodb://prisma:password-with-#@localhost:27017/test";

    let error = RT
        .block_on(async move { MongoDbIntrospectionConnector::new(conn_str).await })
        .unwrap_err();

    let error = error.user_facing_error().cloned().unwrap();
    let error = user_facing_errors::Error::from(error);
    let json_error = serde_json::to_string_pretty(&error).unwrap();

    let expected = expect![[r#"
        {
          "is_panic": false,
          "message": "The provided database string is invalid. invalid port number in database URL. Please refer to the documentation in https://www.prisma.io/docs/reference/database-reference/connection-urls for constructing a correct connection string. In some cases, certain characters must be escaped. Please check the string for any illegal characters.",
          "meta": {
            "details": "invalid port number in database URL. Please refer to the documentation in https://www.prisma.io/docs/reference/database-reference/connection-urls for constructing a correct connection string. In some cases, certain characters must be escaped. Please check the string for any illegal characters."
          },
          "error_code": "P1013"
        }"#]];

    expected.assert_eq(&json_error);
}

#[test]
fn using_without_preview_feature_enabled() {
    let error = RT
        .block_on(async move {
            let dml = indoc::formatdoc!(
                r#"
                    datasource db {{
                      provider = "mongodb"
                      url      = "{}"
                    }}
                "#,
                &*CONN_STR
            );

            let mut config = datamodel::parse_configuration(&dml).unwrap();

            let ctx = IntrospectionContext {
                source: config.subject.datasources.pop().unwrap(),
                composite_type_depth: Default::default(),
                preview_features: Default::default(),
            };

            let connector = MongoDbIntrospectionConnector::new(&*CONN_STR).await?;
            connector.introspect(&Datamodel::new(), ctx).await
        })
        .unwrap_err();

    let error = error.user_facing_error().cloned().unwrap();
    let error = user_facing_errors::Error::from(error);
    let json_error = serde_json::to_string_pretty(&error).unwrap();

    let expected = expect![[r#"
        {
          "is_panic": false,
          "message": "Preview feature not enabled: MongoDB introspection connector (experimental feature, needs to be enabled)",
          "meta": {
            "message": "Preview feature not enabled: MongoDB introspection connector (experimental feature, needs to be enabled)"
          },
          "error_code": "P1019"
        }"#]];

    expected.assert_eq(&json_error);
}
