mod brin;
mod extensions;
mod gin;
mod gist;
mod spgist;

use indoc::indoc;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn sequences_should_work(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE SEQUENCE "first_Sequence";
        CREATE SEQUENCE "second_sequence";
        CREATE SEQUENCE "third_Sequence";
 
        CREATE TABLE "Test" (
            id INTEGER PRIMARY KEY,
            serial Serial,
            first BigInt NOT NULL DEFAULT nextval('"first_Sequence"'::regclass),
            second  BigInt Default nextval('"second_sequence"'),
            third  BigInt Not Null Default nextval('"third_Sequence"'::text)
        );
    "#;

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Test {
          id     Int     @id
          serial Int     @default(autoincrement())
          first  BigInt  @default(autoincrement())
          second BigInt? @default(autoincrement())
          third  BigInt  @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&expectation).await;

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

#[test_connector(tags(Postgres), exclude(CockroachDb))]
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

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model defaults {
          id                String     @id
          text_empty        String[]   @default([])
          text              String[]   @default(["abc"])
          text_c_escape     String[]   @default(["abc", "def"])
          colors            color[]    @default([RED, GREEN])
          int_defaults      Int[]      @default([9, 12999, -4, 0, 1249849])
          float_defaults    Float[]    @default([0, 9.12, 3.14, 0.1242, 124949.124949])
          bool_defaults     Boolean[]  @default([true, true, true, false])
          datetime_defaults DateTime[] @default(dbgenerated("'{\"2022-09-01 08:00:00+00\",\"2021-09-01 08:00:00+00\"}'::timestamp with time zone[]")) @db.Timestamptz
        }

        enum color {
          RED
          GREEN
          BLUE
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}
