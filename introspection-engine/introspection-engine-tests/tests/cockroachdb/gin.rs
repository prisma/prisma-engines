use expect_test::expect;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(CockroachDb), preview_features("cockroachDb"))]
async fn gin_preview_disabled(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data int[] not null)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Int[]

          @@index([data])
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(CockroachDb), preview_features("extendedIndexes", "cockroachDb"))]
async fn gin_unsupported_type(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data geometry not null)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int                     @id @default(autoincrement())
          data Unsupported("geometry")

          @@index([data], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(CockroachDb), preview_features("extendedIndexes", "cockroachDb"))]
async fn array_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data int[] not null)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Int[]

          @@index([data], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(CockroachDb), preview_features("extendedIndexes", "cockroachDb"))]
async fn jsonb_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();

    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data jsonb not null)",);

    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Json

          @@index([data], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
