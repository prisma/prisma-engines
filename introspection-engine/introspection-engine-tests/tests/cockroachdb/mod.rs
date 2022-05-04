mod gin;

use datamodel::parse_configuration;
use introspection_connector::{CompositeTypeDepth, IntrospectionConnector, IntrospectionContext};
use introspection_engine_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
async fn introspecting_cockroach_db_with_postgres_provider(api: TestApi) {
    let setup = r#"
        CREATE TABLE "myTable" (
            id   INTEGER PRIMARY KEY,
            name STRING
       );
    "#;

    let schema = format!(
        r#"
        datasource mypg {{
            provider = "postgresql"
            url = "{}"
        }}

    "#,
        api.connection_string()
    );

    api.raw_cmd(setup).await;

    let ctx = IntrospectionContext {
        preview_features: Default::default(),
        source: parse_configuration(&schema)
            .unwrap()
            .subject
            .datasources
            .into_iter()
            .next()
            .unwrap(),
        composite_type_depth: CompositeTypeDepth::Infinite,
    };

    api.api
        .introspect(&datamodel::parse_datamodel(&schema).unwrap().subject, ctx)
        .await
        .unwrap();
}
