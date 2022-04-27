use expect_test::expect;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb, Postgres9))]
async fn spgist_preview_disabled(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();

    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING SPGIST (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data])
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
async fn spgist_raw_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data box)",);
    let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING SPGIST (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   Int                 @id @default(autoincrement())
          data Unsupported("box")?

          @@index([data], type: SpGist)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb, Postgres9), preview_features("extendedIndexes"))]
async fn spgist_inet_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING SPGIST (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data], type: SpGist)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
async fn spgist_text_ops(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data text)",);
    let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING SPGIST (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String?

          @@index([data], type: SpGist)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
async fn spgist_text_ops_varchar(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data varchar(420))",);
    let create_primary = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING SPGIST (data);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.VarChar(420)

          @@index([data], type: SpGist)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
