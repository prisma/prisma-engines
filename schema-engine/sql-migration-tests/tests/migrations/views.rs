use indoc::indoc;
use sql_migration_tests::test_api::*;

#[test_connector(preview_features("views"))]
fn nothing_gets_written_in_migrations(api: TestApi) {
    let dm = indoc! {r#"
        generator js {
          provider = "prisma-client-javascript"
          previewFeatures = ["views"]
        }

        view Mountain {
          id   Int    @unique
          name String
        }
    "#};

    let expected_sql = expect!["-- This is an empty migration."];
    api.expect_sql_for_schema(dm, &expected_sql);
}
