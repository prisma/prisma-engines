use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_minimal_view(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        view Money {
          id Int @unique
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
        }

        view Money {
          id Int @unique
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_docs(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        /// Foo
        view Money {
          /// Bar
          id Int @unique
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
        }

        /// Foo
        view Money {
          /// Bar
          id Int @unique
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_mapping(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        view Money {
          id Int @unique

          @@map("_money")
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
        }

        view Money {
          id Int @unique

          @@map("_money")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_id(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        view Money {
          id Int
          jd Int

          @@id([id, jd], name: "asdf", map: "Pkey")
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
        }

        view Money {
          id Int
          jd Int

          @@id([id, jd], name: "asdf", map: "Pkey")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        view Money {
          id Int
          jd Int

          @@unique([id, jd], name: "asdf", map: "Pkey")
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
        }

        view Money {
          id Int
          jd Int

          @@unique([id, jd], name: "asdf", map: "Pkey")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        view Money {
          id Int @id
          jd Int

          @@index([id, jd], map: "Pkey")
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
        }

        view Money {
          id Int @id
          jd Int

          @@index([id, jd], map: "Pkey")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_ignore(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        view Money {
          id Int
          jd Int

          @@ignore
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
        }

        view Money {
          id Int
          jd Int

          @@ignore
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_introspection_forward_relation(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model User {
          id Int @id @default(autoincrement())
          moneys Money[]
        }

        view Money {
          id Int @unique
          user_id Int

          user User @relation(fields: [user_id], references: [id])
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id Int @id @default(autoincrement())
          moneys Money[]
        }

        view Money {
          id Int @unique
          user_id Int

          user User @relation(fields: [user_id], references: [id])
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}
