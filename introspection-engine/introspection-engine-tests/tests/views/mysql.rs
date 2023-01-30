use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("views"))]
async fn simple_view_from_one_table(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (
            id INT PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW B AS SELECT id, first_name, last_name FROM A;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id         Int     @id
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"[]"#]];
    api.expect_warnings(&expected).await;

    Ok(())
}
