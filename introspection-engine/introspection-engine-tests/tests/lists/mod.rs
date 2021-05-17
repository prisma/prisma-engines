use barrel::types;
use indoc::formatdoc;
use introspection_engine_tests::{test_api::*, TestResult};
use test_macros::test_connector;

#[test_connector(capabilities(ScalarLists))]
async fn scalar_list_types(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("ints", types::custom("integer[12]"));
                t.add_column("bools", types::custom("boolean[12]"));
                t.add_column("strings", types::custom("text[12]"));
                t.add_column("floats", types::custom("float[12]"));
            });
        })
        .await?;

    let dm = formatdoc! {r#"
         model Post {{
            id       {int_type} @id @default(autoincrement())
            ints     {int_type} []
            bools    Boolean []
            strings  String []
            floats   Float []
         }}
    "#, int_type = api.int_type()};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}
