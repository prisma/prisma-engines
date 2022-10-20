use expect_test::expect;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Mysql))]
async fn remapping_enum_names(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `123Book` (
          id INT NOT NULL AUTO_INCREMENT,
          1color ENUM ('black') NULL,
          PRIMARY KEY (id)
        )
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Book {
          id    Int         @id @default(autoincrement())
          color Book_color? @map("1color")

          @@map("123Book")
        }

        enum Book_color {
          black

          @@map("123Book_1color")
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}
