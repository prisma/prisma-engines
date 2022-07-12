use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mysql))]
async fn an_enum_with_invalid_value_names_should_have_them_commented_out(api: &TestApi) -> TestResult {
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

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
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
