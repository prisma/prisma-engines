use crate::*;
use barrel::types;
use pretty_assertions::assert_eq;

#[test_each_connector()]
async fn introspecting_a_table_with_reserved_name_should_rename(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Transaction", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let dm = "/// This model has been renamed to \'RenamedTransaction\' during introspection, because the original name \'Transaction\' is reserved.\nmodel RenamedTransaction {\n  id Int @default(autoincrement()) @id\n\n  @@map(\"Transaction\")\n}\n";
    let result = api.introspect().await;

    assert_eq!(&result, dm);
}

#[test_each_connector()]
async fn reserved_names_case_sensitivity(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("query", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let dm = "model query {\n  id Int @default(autoincrement()) @id\n}\n";
    let result = api.introspect().await;

    assert_eq!(&result, dm);
}
