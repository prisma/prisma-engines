use sql_introspection_tests::test_api::*;

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn an_enum_with_invalid_value_names_should_have_them_commented_out(api: &mut TestApi) -> TestResult {
    let sql = r#"CREATE TABLE `test` ( `threechars` ENUM ('123', 'wow','$§!') );"#;
    api.raw_cmd(sql).await;
    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model test {
          threechars test_threechars?

          @@ignore
        }

        enum test_threechars {
          // 123 @map("123")
          wow
          // $§! @map("$§!")
        }
    "#]];
    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn a_table_with_an_enum_default_value_that_is_an_empty_string(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `Book` (
            id INT NOT NULL AUTO_INCREMENT,
            color ENUM ('black', '') NOT NULL DEFAULT '',
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
          id    Int        @id @default(autoincrement())
          color Book_color @default(EMPTY_ENUM_VALUE)
        }

        enum Book_color {
          black
          EMPTY_ENUM_VALUE @map("")
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn a_table_with_enum_default_values_that_look_like_booleans(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `News` (
            id INT NOT NULL AUTO_INCREMENT,
            confirmed ENUM ('true', 'false', 'rumor') NOT NULL DEFAULT 'true',
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

        model News {
          id        Int            @id @default(autoincrement())
          confirmed News_confirmed @default(true)
        }

        enum News_confirmed {
          true
          false
          rumor
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}
