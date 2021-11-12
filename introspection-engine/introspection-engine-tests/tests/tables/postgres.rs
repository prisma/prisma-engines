use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Postgres), preview_features("extendedIndexes"))]
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

#[test_connector(tags(Postgres), preview_features("extendedIndexes"))]
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

#[test_connector(tags(Postgres), preview_features("extendedIndexes"))]
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
