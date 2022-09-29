use introspection_engine_tests::{test_api::*, TestResult};
use test_macros::test_connector;

//Todo(matthias) not there yet
// #[test_connector(tags(Postgres), preview_features("multiSchema"))]
// async fn multiple_schemas_are_introspected(api: &TestApi) -> TestResult {
//     let schema_name = api.schema_name();
//     let other_name = "test";
//     let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data Integer)",);
//     let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" (\"data\")",);
//
//     api.database().raw_cmd(&create_table).await?;
//     api.database().raw_cmd(&create_primary).await?;
//
//     let create_schema = format!("CREATE Schema \"{other_name}\"",);
//     let create_table = format!("CREATE TABLE \"{other_name}\".\"A\" (id SERIAL PRIMARY KEY, data Integer)",);
//     let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{other_name}\".\"A\" (\"data\")",);
//
//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_table).await?;
//     api.database().raw_cmd(&create_primary).await?;
//
//     let expected = expect![[r#"
//         model A {
//           id   Int                 @id @default(autoincrement())
//           data Unsupported("box")?
//
//           @@index([data], type: SpGist)
//         }
//     "#]];
//
//     let result = api.introspect_dml().await?;
//     expected.assert_eq(&result);
//
//     Ok(())
// }
