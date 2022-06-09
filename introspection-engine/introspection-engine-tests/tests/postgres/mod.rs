mod brin;
mod gin;
mod gist;
mod spgist;

use indoc::{formatdoc, indoc};
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn sequences_should_work(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.inject_custom("CREATE SEQUENCE \"first_Sequence\"");
            migration.inject_custom("CREATE SEQUENCE \"second_sequence\"");
            migration.inject_custom("CREATE SEQUENCE \"third_Sequence\"");

            migration.create_table("Test", move |t| {
                t.inject_custom("id Integer Primary Key");
                t.inject_custom("serial  Serial");
                t.inject_custom("first   BigInt Not Null Default nextval('\"first_Sequence\"'::regclass)");
                t.inject_custom("second  BigInt Default nextval('\"second_sequence\"')");
                t.inject_custom("third  BigInt Not Null Default nextval('\"third_Sequence\"'::text)");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
          id     Int        @id
          serial Int        @default(autoincrement())
          first  BigInt     @default(autoincrement())
          second BigInt?    @default(autoincrement())
          third  BigInt     @default(autoincrement())
        }
    "#};

    let with_ds = formatdoc!(
        r#"
        datasource ds {{
          provider = "postgres"
          url = "postgres://"
        }}

        {}
    "#,
        dm
    );

    let result = api.re_introspect(&with_ds).await?;

    println!("EXPECTATION: \n {:#}", dm);
    println!("RESULT: \n {:#}", result);

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn dbgenerated_type_casts_should_work(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("A", move |t| {
                t.inject_custom("id VARCHAR(30) PRIMARY KEY DEFAULT (now())::text");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model A {
          id String @id @default(dbgenerated("(now())::text")) @db.VarChar(30)
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn pg_xml_indexes_are_skipped(api: &TestApi) -> TestResult {
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".xml_test (id SERIAL PRIMARY KEY, data XML)",
        schema_name = api.schema_name()
    );

    let create_primary = format!(
        "CREATE INDEX test_idx ON \"{schema_name}\".xml_test USING BTREE (cast(xpath('/book/title', data) as text[]));",
        schema_name = api.schema_name(),
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let dm = indoc! {r#"
        model xml_test {
          id   Int @id @default(autoincrement())
          data String? @db.Xml
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn scalar_list_defaults_work(api: &TestApi) -> TestResult {
    let schema = r#"
        CREATE TYPE "color" AS ENUM ('RED', 'GREEN', 'BLUE');

        CREATE TABLE "defaults" (
            id TEXT PRIMARY KEY,
            text_empty TEXT[] NOT NULL DEFAULT '{}',
            text TEXT[] NOT NULL DEFAULT '{ ''abc'' }',
            text_c_escape TEXT[] NOT NULL DEFAULT E'{ \'abc\', \'def\' }',
            colors COLOR[] NOT NULL DEFAULT '{ RED, GREEN }',
            int_defaults INT4[] NOT NULL DEFAULT '{ 9, 12999, -4, 0, 1249849 }',
            float_defaults DOUBLE PRECISION[] NOT NULL DEFAULT '{ 0, 9.12, 3.14, 0.1242, 124949.124949 }',
            bool_defaults BOOLEAN[] NOT NULL DEFAULT '{ true, true, true, false }',
            datetime_defaults TIMESTAMPTZ[] NOT NULL DEFAULT '{ ''2022-09-01T08:00Z'',''2021-09-01T08:00Z''}'
        );
    "#;

    api.raw_cmd(schema).await;

    let expectation = expect![[]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}
