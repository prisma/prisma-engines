use introspection_engine_tests::{test_api::*, TestResult};
use test_macros::test_connector;

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    db_schemas("first", "second")
)]
async fn multiple_schemas_are_introspected(api: &TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE Schema \"{schema_name}\"",);
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data Integer)",);
    let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" (\"data\")",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let create_schema = format!("CREATE Schema \"{other_name}\"",);
    let create_table = format!("CREATE TABLE \"{other_name}\".\"A\" (id SERIAL PRIMARY KEY, data Integer)",);
    let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{other_name}\".\"A\" (\"data\")",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    //TODO(matthias) Not correct yet, just a POC for the db_schemas property in the test macro
    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int?
        
          @@index([data])
        }
        
        model A {
          id   Int  @id @default(autoincrement())
          data Int?
        
          @@index([data])
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
