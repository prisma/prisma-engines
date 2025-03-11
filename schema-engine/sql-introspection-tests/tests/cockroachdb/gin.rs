use sql_introspection_tests::test_api::*;

#[test_connector(tags(CockroachDb), preview_features("cockroachDb"))]
async fn gin_unsupported_type(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();

    let setup = formatdoc!(
        r#"
        CREATE TABLE "{schema_name}"."A" (
            id SERIAL PRIMARY KEY,
            data geometry NOT NULL
        );

        CREATE INDEX "A_data_idx"
                  ON "{schema_name}"."A"
               USING GIN (data);
    "#
    );

    api.database().raw_cmd(&setup).await?;

    let expected = expect![[r#"
        model A {
          id   BigInt                  @id @default(autoincrement())
          data Unsupported("geometry")

          @@index([data], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(CockroachDb), preview_features("cockroachDb"))]
async fn array_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();

    let setup = formatdoc!(
        r#"
        CREATE TABLE "{schema_name}"."A" (
            id SERIAL PRIMARY KEY,
            data int[] NOT NULL
        );

        CREATE INDEX "A_data_idx"
                  ON "{schema_name}"."A"
               USING GIN (data);
    "#
    );

    api.database().raw_cmd(&setup).await?;

    let expected = expect![[r#"
        model A {
          id   BigInt   @id @default(autoincrement())
          data BigInt[]

          @@index([data], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(CockroachDb), preview_features("cockroachDb"))]
async fn jsonb_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();

    let setup = formatdoc!(
        r#"
        CREATE TABLE "{schema_name}"."A" (
            id SERIAL PRIMARY KEY,
            data jsonb NOT NULL
        );

        CREATE INDEX "A_data_idx"
                  ON "{schema_name}"."A"
               USING GIN (data);
    "#
    );

    api.database().raw_cmd(&setup).await?;

    let expected = expect![[r#"
        model A {
          id   BigInt @id @default(autoincrement())
          data Json

          @@index([data], type: Gin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
