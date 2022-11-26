use introspection_engine_tests::test_api::*;

#[test_connector(tags(Postgres))]
async fn enum_reintrospection_preserves_good_indentation(api: &TestApi) -> TestResult {
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
async fn a_table_enums_array(api: &TestApi) -> TestResult {
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
          provider = "prisma-client-js"
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
async fn an_enum_with_invalid_value_names_should_have_them_commented_out(api: &TestApi) -> TestResult {
    let sql = r#"CREATE TYPE "threechars" AS ENUM ('123', 'wow','$§!');"#;
    api.raw_cmd(sql).await;
    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
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
async fn a_table_with_an_enum_default_value_that_is_an_empty_string(api: &TestApi) -> TestResult {
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
          provider = "prisma-client-js"
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
async fn a_table_with_enum_default_values_that_look_like_booleans(api: &TestApi) -> TestResult {
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
          provider = "prisma-client-js"
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
async fn invalid_enum_variants_regression(api: &TestApi) -> TestResult {
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
          provider = "prisma-client-js"
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
        [
          {
            "code": 4,
            "message": "These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute.",
            "affected": [
              {
                "enm": "invalid_enum",
                "value": "123"
              },
              {
                "enm": "invalid_enum",
                "value": "$§!"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}
