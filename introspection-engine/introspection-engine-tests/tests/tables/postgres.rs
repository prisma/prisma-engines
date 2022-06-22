use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn string_defaults_that_need_escaping(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "stringstest" (
            id INTEGER PRIMARY KEY,
            needs_escaping TEXT NOT NULL DEFAULT $$
abc def
backspaces: \abcd\
	(tab character)
and "quotes" and a vertical tabulation here -><-

$$
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model stringstest {
          id             Int    @id
          needs_escaping String @default("\nabc def\nbackspaces: \\abcd\\\n\t(tab character)\nand \"quotes\" and a vertical tabulation here ->\u0016<-\n\n")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn a_table_with_descending_unique(api: &TestApi) -> TestResult {
    let setup = r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER NOT NULL,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE UNIQUE INDEX "A_a_key" ON "A" (a DESC);
   "#;

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id(map: "a_pkey")
          a  Int @unique(sort: Desc)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn a_table_with_descending_compound_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER NOT NULL,
           b  INTEGER NOT NULL,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE UNIQUE INDEX "A_a_b_key" ON "A" (a ASC, b DESC);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id(map: "a_pkey")
          a  Int
          b  Int

          @@unique([a, b(sort: Desc)])
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn a_table_with_descending_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER NOT NULL,
           b  INTEGER NOT NULL,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE INDEX "A_a_b_idx" ON "A" (a ASC, b DESC);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id(map: "a_pkey")
          a  Int
          b  Int

          @@index([a, b(sort: Desc)])
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_a_hash_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE INDEX "A_a_idx" ON "A" USING HASH (a);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int  @id(map: "a_pkey")
          a  Int?

          @@index([a], type: Hash)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn ignoring_of_partial_indices(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE INDEX "A_a_idx" ON "A" Using Btree (a) Where (a is not null);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int  @id(map: "a_pkey")
          a  Int?
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn introspecting_now_functions(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL Primary Key,
           timestamp Timestamp Default now(),
           timestamp_tz Timestamp with time zone Default now(),
           date date Default now(),
           timestamp_2 Timestamp Default current_timestamp,
           timestamp_tz_2 Timestamp with time zone Default current_timestamp,
           date_2 date Default current_timestamp
        );

       "#};
    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id             Int       @id
          timestamp      DateTime? @default(now()) @db.Timestamp(6)
          timestamp_tz   DateTime? @default(now()) @db.Timestamptz(6)
          date           DateTime? @default(now()) @db.Date
          timestamp_2    DateTime? @default(now()) @db.Timestamp(6)
          timestamp_tz_2 DateTime? @default(now()) @db.Timestamptz(6)
          date_2         DateTime? @default(now()) @db.Date
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

// https://github.com/prisma/prisma/issues/12095
#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_json_columns(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            "bar" JSONB DEFAULT '{"message": "This message includes a quote: Here''s it!"}',

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Foo {
          id  Int   @id
          bar Json? @default("{\"message\": \"This message includes a quote: Here's it!\"}")
        }
    "#]];

    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn datetime_default_expressions_are_not_truncated(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            trial_expires TIMESTAMPTZ(6) NOT NULL DEFAULT now()::TIMESTAMPTZ + '14 days'::INTERVAL,

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Foo {
          id            Int      @id
          trial_expires DateTime @default(dbgenerated("(now() + '14 days'::interval)")) @db.Timestamptz(6)
        }
    "#]];

    api.expect_datamodel(&expected).await;
    Ok(())
}
