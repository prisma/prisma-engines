use indoc::indoc;
use sql_migration_tests::test_api::*;

#[test_connector(preview_features("views"))]
fn views_are_ignored(api: TestApi) {
    let dm = indoc! {r#"
        view Dog {
          val Int
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
    api.assert_schema().assert_has_no_table("Dog").assert_has_no_view("Dog");
}

#[test_connector(preview_features("views"))]
fn relations_from_view_are_ignored(api: TestApi) {
    let dm = indoc! {r#"
        model Leash {
          id   Int   @id
          dogs Dog[]
        }

        view Dog {
          val     Int   @unique
          leashId Int
          leash   Leash @relation(fields: [leashId], references: [id])
        }
    "#};

    api.schema_push_w_datasource(dm)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Leash", |table| table.assert_no_fks());
}

#[test_connector(preview_features("views"))]
fn relations_to_view_are_ignored(api: TestApi) {
    let dm = indoc! {r#"
        model Leash {
          id    Int @id
          dogId Int
          dog   Dog @relation(fields: [dogId], references: [val])
        }

        view Dog {
          val     Int     @unique
          leashes Leash[]
        }
    "#};

    api.schema_push_w_datasource(dm)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Leash", |table| table.assert_no_fks());
}
