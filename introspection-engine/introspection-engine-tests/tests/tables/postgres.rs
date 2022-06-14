use barrel::types;
use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Postgres))]
async fn a_table_with_descending_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER NOT NULL,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE UNIQUE INDEX "A_a_key" ON "A" (a DESC);
   "#};

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

#[test_connector(tags(CockroachDb))]
async fn introspecting_json_defaults_on_cockroach(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL Primary Key,
           json Json Default '[]'::json,
           jsonb JsonB Default '{}'::jsonb
         );

       "#};
    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id    Int   @id
          json  Json? @default("[]")
          jsonb Json? @default("{}")
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspecting_a_table_with_json_type_must_work(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("json", types::json());
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Blog {
            id      Int @id @default(autoincrement())
            json    Json @db.Json
        }
    "#};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}
