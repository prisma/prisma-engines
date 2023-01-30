use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Sqlite), preview_features("views"))]
async fn simple_view_from_one_table(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (
            id INT NOT NULL PRIMARY KEY,
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
          provider = "sqlite"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id         Int     @id
          first_name String
          last_name  String?
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"[]"#]];
    api.expect_warnings(&expected).await;

    Ok(())
}
