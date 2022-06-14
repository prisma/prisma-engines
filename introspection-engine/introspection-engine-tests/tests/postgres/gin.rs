use expect_test::expect;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn full_text_functions_filtered_out(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data text not null)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (to_tsvector('english', data));",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn gin_raw_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data tsvector not null)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int                     @id @default(autoincrement())
          data Unsupported("tsvector")

          @@index([data], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn jsonb_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();

    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data jsonb not null)",);

    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data jsonb_ops);",);

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

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn jsonb_path_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();

    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data jsonb not null)",);

    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data jsonb_path_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Json

          @@index([data(ops: JsonbPathOps)], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
