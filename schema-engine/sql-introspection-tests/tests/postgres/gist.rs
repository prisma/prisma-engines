use expect_test::expect;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn gist_inet_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIST (data inet_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data(ops: InetOps)], type: Gist)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn gist_raw_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data tsvector)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIST (data tsvector_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int                      @id @default(autoincrement())
          data Unsupported("tsvector")?

          @@index([data], type: Gist)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
