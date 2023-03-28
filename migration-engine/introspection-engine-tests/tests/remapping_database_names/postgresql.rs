use expect_test::expect;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn remapping_enum_names(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TYPE "123color" AS ENUM ('black');

        CREATE TABLE "123Book" (
          id SERIAL PRIMARY KEY,
          "1color" "123color" NULL
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

        model Book {
          id    Int    @id @default(autoincrement())
          color color? @map("1color")

          @@map("123Book")
        }

        enum color {
          black

          @@map("123color")
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}
