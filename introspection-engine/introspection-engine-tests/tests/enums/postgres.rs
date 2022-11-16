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
