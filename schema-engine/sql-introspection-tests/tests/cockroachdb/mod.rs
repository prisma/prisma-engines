mod constraints;
mod gin;

use indoc::indoc;
use schema_connector::{CompositeTypeDepth, ConnectorParams, IntrospectionContext, SchemaConnector};
use sql_introspection_tests::test_api::*;
use sql_schema_connector::SqlSchemaConnector;

#[test_connector(tags(CockroachDb))]
async fn introspecting_cockroach_db_with_postgres_provider_fails(api: TestApi) {
    let setup = r#"
        CREATE TABLE "myTable" (
            id   INTEGER PRIMARY KEY,
            name STRING
       );
    "#;

    let schema = format!(
        r#"
        datasource mypg {{
            provider = "postgresql"
            url = "{}"
        }}

    "#,
        api.connection_string()
    );

    api.raw_cmd(setup).await;

    let schema = psl::parse_schema(schema).unwrap();
    let ctx = IntrospectionContext::new_config_only(schema, CompositeTypeDepth::Infinite, None);

    // Instantiate the schema connector manually for this test because `TestApi`
    // chooses the provider type based on the current database under test and
    // not on the `provider` field in the schema.
    let mut engine = SqlSchemaConnector::new_postgres();
    let params = ConnectorParams {
        connection_string: api.connection_string().to_owned(),
        preview_features: api.preview_features(),
        shadow_database_connection_string: None,
    };
    engine.set_params(params).unwrap();

    let err = engine.introspect(&ctx).await.unwrap_err().to_string();

    let expected_err = expect![[r#"
        You are trying to connect to a CockroachDB database, but the provider in your Prisma schema is `postgresql`. Please change it to `cockroachdb`.
    "#]];

    expected_err.assert_eq(&err);
}

#[test_connector(tags(CockroachDb))]
async fn rowid_introspects_to_autoincrement(api: TestApi) {
    let sql = r#"
    CREATE TABLE "myTable"(
        id   INT4 PRIMARY KEY DEFAULT unique_rowid(),
        name STRING NOT NULL
    );
    "#;

    api.raw_cmd(sql).await;

    let result = api.introspect_dml().await.unwrap();

    let expected = expect![[r#"
        model myTable {
          id   Int    @id @default(autoincrement())
          name String
        }
    "#]];

    expected.assert_eq(&result);
}

#[test_connector(tags(CockroachDb221))]
async fn identity_introspects_to_sequence_with_default_settings_v_22_1(api: TestApi) {
    let sql = r#"
    CREATE TABLE "myTable" (
        id   INT4 GENERATED BY DEFAULT AS IDENTITY,
        name STRING NOT NULL,

        PRIMARY KEY (id)
    );
    "#;

    api.raw_cmd(sql).await;

    let result = api.introspect_dml().await.unwrap();

    let expected = expect![[r#"
        model myTable {
          id   Int    @id @default(sequence())
          name String
        }
    "#]];

    expected.assert_eq(&result);
}

#[test_connector(tags(CockroachDb222))]
async fn identity_introspects_to_sequence_with_default_settings_v_22_2(api: TestApi) {
    let sql = r#"
    CREATE TABLE "myTable" (
        id   INT4 GENERATED BY DEFAULT AS IDENTITY,
        name STRING NOT NULL,

        PRIMARY KEY (id)
    );
    "#;

    api.raw_cmd(sql).await;

    let result = api.introspect_dml().await.unwrap();

    let expected = expect![[r#"
        model myTable {
          id   Int    @id @default(sequence(maxValue: 2147483647))
          name String
        }
    "#]];

    expected.assert_eq(&result);
}

#[test_connector(tags(CockroachDb))]
async fn identity_with_options_introspects_to_sequence_with_options(api: TestApi) {
    let sql = r#"
    CREATE TABLE "myTable" (
        id   INT4 GENERATED BY DEFAULT AS IDENTITY (MINVALUE 10 START 12 MAXVALUE 39 INCREMENT 3 CACHE 4),
        name STRING NOT NULL,

        PRIMARY KEY (id)
    );
    "#;

    api.raw_cmd(sql).await;

    let result = api.introspect_dml().await.unwrap();

    let expected = expect![[r#"
        model myTable {
          id   Int    @id @default(sequence(minValue: 10, maxValue: 39, cache: 4, increment: 3, start: 12))
          name String
        }
    "#]];

    expected.assert_eq(&result);
}

#[test_connector(tags(CockroachDb))]
async fn dbgenerated_type_casts_should_work(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("A", move |t| {
                t.inject_custom("id VARCHAR(30) PRIMARY KEY DEFAULT (now())::text");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model A {
          id String @id @default(dbgenerated("now()::STRING")) @db.String(30)
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(CockroachDb221))]
async fn scalar_list_defaults_work_on_22_1(api: &mut TestApi) -> TestResult {
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
            datetime_defaults TIMESTAMPTZ[] NOT NULL DEFAULT '{ "2022-09-01T08:00Z","2021-09-01T08:00Z"}'
        );
    "#;

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
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
          datetime_defaults DateTime[] @default(dbgenerated("'{\"''2022-09-01 08:00:00+00:00''::TIMESTAMPTZ\",\"''2021-09-01 08:00:00+00:00''::TIMESTAMPTZ\"}'::TIMESTAMPTZ[]")) @db.Timestamptz
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

#[test_connector(tags(CockroachDb222))]
async fn scalar_list_defaults_work_on_22_2(api: &mut TestApi) -> TestResult {
    let schema = r#"
        CREATE TYPE "color" AS ENUM ('RED', 'GREEN', 'BLUE');

        CREATE TABLE "defaults" (
            id TEXT PRIMARY KEY,
            text_empty TEXT[] NOT NULL DEFAULT '{}',
            text TEXT[] NOT NULL DEFAULT '{ ''abc'' }',
            text_c_escape TEXT[] NOT NULL DEFAULT E'{ \'abc\', \'def\' }',
            colors COLOR[] NOT NULL DEFAULT '{ RED, GREEN }',
            int_defaults INT4[] NOT NULL DEFAULT '{ 9, 12999, -4, 0, 1249849 }',
            float_defaults DOUBLE PRECISION[] NOT NULL DEFAULT '{ 0.0, 9.12, 3.14, 0.1242, 124949.124949 }',
            bool_defaults BOOLEAN[] NOT NULL DEFAULT '{ true, true, true, false }',
            datetime_defaults TIMESTAMPTZ[] NOT NULL DEFAULT '{ "2022-09-01T08:00Z","2021-09-01T08:00Z"}'
        );
    "#;

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
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
          datetime_defaults DateTime[] @default(dbgenerated("'{\"''2022-09-01 08:00:00+00:00''::TIMESTAMPTZ\",\"''2021-09-01 08:00:00+00:00''::TIMESTAMPTZ\"}'::TIMESTAMPTZ[]")) @db.Timestamptz
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

#[test_connector(tags(CockroachDb))]
async fn string_col_with_length(api: &mut TestApi) -> TestResult {
    let schema = r#"
        CREATE TABLE "User" (
          id INT8 PRIMARY KEY DEFAULT unique_rowid(),
          name STRING(255),
          email STRING(255) UNIQUE NOT NULL
        );

        CREATE TABLE "Post" (
          id INT8 PRIMARY KEY DEFAULT unique_rowid(),
          title STRING(255) UNIQUE NOT NULL,
          "createdAt" TIMESTAMP NOT NULL DEFAULT now(),
          content STRING,
          published BOOLEAN NOT NULL DEFAULT false,
          "authorId" INT8 NOT NULL,
          FOREIGN KEY ("authorId") REFERENCES "User"(id)
        );

        CREATE TABLE "Profile" (
          id INT8 PRIMARY KEY DEFAULT unique_rowid(),
          bio STRING,
          "userId" INT8 UNIQUE NOT NULL,
          FOREIGN KEY ("userId") REFERENCES "User"(id)
        );
    "#;

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Post {
          id        BigInt   @id @default(autoincrement())
          title     String   @unique @db.String(255)
          createdAt DateTime @default(now()) @db.Timestamp(6)
          content   String?
          published Boolean  @default(false)
          authorId  BigInt
          User      User     @relation(fields: [authorId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Profile {
          id     BigInt  @id @default(autoincrement())
          bio    String?
          userId BigInt  @unique
          User   User    @relation(fields: [userId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id      BigInt   @id @default(autoincrement())
          name    String?  @db.String(255)
          email   String   @unique @db.String(255)
          Post    Post[]
          Profile Profile?
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn row_level_ttl_stopgap(api: &mut TestApi) -> TestResult {
    // https://www.notion.so/prismaio/Row-level-TTL-CockroachDB-87c673e7a14a419aa91ebcd5d16d227b
    //

    let schema = indoc! {r#"
        CREATE TABLE "ttl_test" (
            id SERIAL PRIMARY KEY,
            inserted_at TIMESTAMP default current_timestamp()
        ) WITH (ttl_expire_after = '3 months');
    "#};

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This model is using a row level TTL in the database, and requires an additional setup in migrations. Read more: https://pris.ly/d/row-level-ttl
        model ttl_test {
          id          BigInt    @id @default(autoincrement())
          inserted_at DateTime? @default(now()) @db.Timestamp(6)
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These models are using a row level TTL setting defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/row-level-ttl
          - "ttl_test"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! {r#"
        /// This model is using a row level TTL in the database, and requires an additional setup in migrations. Read more: https://pris.ly/d/row-level-ttl
        model ttl_test {
          id          BigInt    @id @default(autoincrement())
          inserted_at DateTime? @default(now()) @db.Timestamp(6)
        }
    "#};

    let expectation = expect![[r#"
        /// This model is using a row level TTL in the database, and requires an additional setup in migrations. Read more: https://pris.ly/d/row-level-ttl
        model ttl_test {
          id          BigInt    @id @default(autoincrement())
          inserted_at DateTime? @default(now()) @db.Timestamp(6)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(CockroachDb), preview_features("views"))]
async fn commenting_stopgap(api: &mut TestApi) -> TestResult {
    // https://www.notion.so/prismaio/Comments-ac89f872098e463183fd668a643f3ab8
    // Only comments on tables and columns are supported.

    let schema = indoc! {r#"
        CREATE TABLE a (
            id INT PRIMARY KEY,
            val VARCHAR(20)
        );

        COMMENT ON TABLE a IS 'push';
        COMMENT ON COLUMN a.val IS 'meow';
    "#};

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This model or at least one of its fields has comments in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        model a {
          id  Int     @id
          val String? @db.String(20)
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These objects have comments defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/database-comments
          - Type: "model", name: "a"
          - Type: "field", name: "a.val"
    "#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}
