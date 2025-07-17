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
