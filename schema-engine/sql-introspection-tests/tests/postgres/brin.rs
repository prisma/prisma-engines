use expect_test::expect;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn bit_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bit)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data bit_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Bit(1)

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn varbit_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data varbit)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data varbit_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.VarBit

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn bpchar_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bpchar)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data bpchar_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Char

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn bpchar_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bpchar)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data bpchar_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Char

          @@index([data(ops: BpcharBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn bytea_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bytea)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data bytea_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Bytes?

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn bytea_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bytea)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data bytea_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Bytes?

          @@index([data(ops: ByteaBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn date_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data date)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data date_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Date

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn date_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data date)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data date_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Date

          @@index([data(ops: DateBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn date_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data date)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data date_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Date

          @@index([data(ops: DateMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn float_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data real)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data float4_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Float? @db.Real

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn float_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data real)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data float4_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Float? @db.Real

          @@index([data(ops: Float4BloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn float_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data real)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data float4_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Float? @db.Real

          @@index([data(ops: Float4MinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn double_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data double precision)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data float8_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Float?

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn double_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data double precision)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data float8_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Float?

          @@index([data(ops: Float8BloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn double_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data double precision)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data float8_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Float?

          @@index([data(ops: Float8MinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn inet_inclusion_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data inet_inclusion_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn inet_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data inet_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data(ops: InetBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn inet_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data inet_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data(ops: InetMinMaxOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn inet_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data inet_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data(ops: InetMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn int2_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data smallint)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int2_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int? @db.SmallInt

          @@index([data(ops: Int2BloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn int2_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data smallint)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int2_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int? @db.SmallInt

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn int2_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data smallint)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int2_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int? @db.SmallInt

          @@index([data(ops: Int2MinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn int4_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data int)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int4_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int?

          @@index([data(ops: Int4BloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn int4_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data int)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int4_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int?

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn int4_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data int)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int4_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int?

          @@index([data(ops: Int4MinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn int8_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bigint)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int8_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data BigInt?

          @@index([data(ops: Int8BloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn int8_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bigint)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int8_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data BigInt?

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn int8_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data bigint)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data int8_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data BigInt?

          @@index([data(ops: Int8MinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn numeric_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data decimal)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data numeric_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int      @id @default(autoincrement())
          data Decimal? @db.Decimal

          @@index([data(ops: NumericBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn numeric_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data decimal)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data numeric_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int      @id @default(autoincrement())
          data Decimal? @db.Decimal

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn numeric_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data decimal)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data numeric_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int      @id @default(autoincrement())
          data Decimal? @db.Decimal

          @@index([data(ops: NumericMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn oid_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data oid)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data oid_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int? @db.Oid

          @@index([data(ops: OidBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn oid_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data oid)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data oid_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int? @db.Oid

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn oid_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data oid)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data oid_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int? @db.Oid

          @@index([data(ops: OidMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn text_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data text)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data text_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String?

          @@index([data(ops: TextBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn text_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data text)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data text_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String?

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn timestamp_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timestamp)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timestamp_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timestamp(6)

          @@index([data(ops: TimestampBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn timestamp_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timestamp)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timestamp_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timestamp(6)

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn timestamp_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timestamp)",);
    let create_idx = format!(
        "CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timestamp_minmax_multi_ops);",
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timestamp(6)

          @@index([data(ops: TimestampMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn timestamptz_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timestamptz)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timestamptz_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timestamptz(6)

          @@index([data(ops: TimestampTzBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn timestamptz_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timestamptz)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timestamptz_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timestamptz(6)

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn timestamptz_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timestamptz)",);
    let create_idx = format!(
        "CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timestamptz_minmax_multi_ops);",
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timestamptz(6)

          @@index([data(ops: TimestampTzMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn time_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data time)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data time_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Time(6)

          @@index([data(ops: TimeBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn time_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data time)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data time_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Time(6)

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn time_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data time)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data time_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Time(6)

          @@index([data(ops: TimeMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn timetz_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timetz)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timetz_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timetz(6)

          @@index([data(ops: TimeTzBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn timetz_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timetz)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timetz_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timetz(6)

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn timetz_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data timetz)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data timetz_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int       @id @default(autoincrement())
          data DateTime? @db.Timetz(6)

          @@index([data(ops: TimeTzMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn uuid_bloom_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data uuid)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data uuid_bloom_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Uuid

          @@index([data(ops: UuidBloomOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn uuid_minmax_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data uuid)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data uuid_minmax_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Uuid

          @@index([data], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
async fn uuid_minmax_multi_ops(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data uuid)",);
    let create_idx =
        format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING BRIN (data uuid_minmax_multi_ops);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Uuid

          @@index([data(ops: UuidMinMaxMultiOps)], type: Brin)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
