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

#[test_connector(preview_features("views"))]
fn creates_no_m2m_relations_to_views(api: TestApi) {
    let dm = indoc! {r#"
        generator js {
          provider = "prisma-client-javascript"
          previewFeatures = ["views"]
        }

        model Organization {
          id        Int        @id
          viewUsers ViewUser[]
        }

        view ViewUser {
          userId        Int            @id @unique
          organizations Organization[]
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_has_no_view("ViewUser");
    api.assert_schema().assert_table("Organization", |table| {
        table.assert_has_column("id").assert_does_not_have_column("viewUsers")
    });
}
