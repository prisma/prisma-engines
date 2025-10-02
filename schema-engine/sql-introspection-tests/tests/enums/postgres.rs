use sql_introspection_tests::test_api::*;

#[test_connector(tags(Postgres))]
async fn enum_reintrospection_preserves_good_indentation(api: &mut TestApi) -> TestResult {
    let original = indoc!(
        r#"
        enum MyEnum {
          A
          B

          @@map("theEnumName")
        }
        "#
    );

    api.raw_cmd(r#"CREATE TYPE "theEnumName" AS ENUM ('A', 'B');"#).await;

    let reintrospected: String = api
        .re_introspect(original)
        .await?
        .lines()
        .skip_while(|l| !l.starts_with("enum"))
        .collect::<Vec<&str>>()
        .join("\n");

    assert_eq!(original.trim_end(), reintrospected);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_enums_array(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TYPE "color" AS ENUM ('black','white');

        CREATE TABLE "Book" (
            id SERIAL PRIMARY KEY,
            color color[] NOT NULL
        );
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Book {
          id    Int     @id @default(autoincrement())
          color color[]
        }

        enum color {
          black
          white
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn an_enum_with_invalid_value_names_should_have_them_commented_out(api: &mut TestApi) -> TestResult {
    let sql = r#"CREATE TYPE "threechars" AS ENUM ('123', 'wow','$§!');"#;
    api.raw_cmd(sql).await;
    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        enum threechars {
          // 123 @map("123")
          wow
          // $§! @map("$§!")
        }
    "#]];
    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(exclude(CockroachDb), tags(Postgres))]
async fn a_table_with_an_enum_default_value_that_is_an_empty_string(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TYPE "color" AS ENUM ('black', '');

        CREATE TABLE "Book" (
            id SERIAL PRIMARY KEY,
            color color NOT NULL DEFAULT ''
        )
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Book {
          id    Int   @id @default(autoincrement())
          color color @default(EMPTY_ENUM_VALUE)
        }

        enum color {
          black
          EMPTY_ENUM_VALUE @map("")
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_enum_default_values_that_look_like_booleans(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE Type truth as ENUM ('true', 'false', 'rumor');

        CREATE TABLE "News" (
            id SERIAL PRIMARY KEY,
            confirmed truth NOT NULL DEFAULT 'true'
        )
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model News {
          id        Int   @id @default(autoincrement())
          confirmed truth @default(true)
        }

        enum truth {
          true
          false
          rumor
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn invalid_enum_variants_regression(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TYPE invalid_enum AS ENUM ('Y','N','123','$§!');

        CREATE TABLE invalid_enum_value_name (
          field1 SERIAL PRIMARY KEY NOT NULL,
          here_be_enum invalid_enum DEFAULT NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model invalid_enum_value_name {
          field1       Int           @id @default(autoincrement())
          here_be_enum invalid_enum?
        }

        enum invalid_enum {
          Y
          N
          // 123 @map("123")
          // $§! @map("$§!")
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:
          - Enum: "invalid_enum", value: "123"
          - Enum: "invalid_enum", value: "$§!"
    "#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_variant_that_cannot_be_sanitized_triggers_dbgenerated_in_defaults(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE Type "A" as ENUM ('0', '1');

        CREATE TABLE "B" (
            id SERIAL PRIMARY KEY,
            val "A" NOT NULL DEFAULT '0'
        )
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model B {
          id  Int @id @default(autoincrement())
          val A   @default(dbgenerated("0"))
        }

        enum A {
          // 0 @map("0")
          // 1 @map("1")
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_mapped_variant_will_not_warn(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE Type "A" as ENUM ('0first', '1second');

        CREATE TABLE "B" (
            id SERIAL PRIMARY KEY,
            val "A" NOT NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model B {
          id  Int @id @default(autoincrement())
          val A
        }

        enum A {
          first  @map("0first")
          second @map("1second")
        }
    "#]];

    api.expect_datamodel(&expectation).await;
    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_mapped_enum_will_not_warn(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE Type "1A" as ENUM ('first', 'second');

        CREATE TABLE "B" (
            id SERIAL PRIMARY KEY,
            val "1A" NOT NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model B {
          id  Int @id @default(autoincrement())
          val A
        }

        enum A {
          first
          second

          @@map("1A")
        }
    "#]];

    api.expect_datamodel(&expectation).await;
    api.expect_no_warnings().await;

    Ok(())
}

// Regression: https://github.com/prisma/prisma/issues/22456
#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn enum_array_type(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TYPE "_foo" AS ENUM ('FIRST', 'SECOND');

        CREATE TABLE "Post" (
            "id" TEXT NOT NULL,
            "contentFilters" "_foo"[],
            CONSTRAINT "Post_pkey" PRIMARY KEY ("id")
        );
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Post {
          id             String @id
          contentFilters foo[]
        }

        enum foo {
          FIRST
          SECOND

          @@map("_foo")
        }
    "#]];

    api.expect_datamodel(&expectation).await;
    api.expect_no_warnings().await;

    Ok(())
}
