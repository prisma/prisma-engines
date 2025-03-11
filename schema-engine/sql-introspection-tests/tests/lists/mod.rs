use barrel::types;
use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(capabilities(ScalarLists))]
async fn scalar_list_types(api: &mut TestApi) -> TestResult {
    api.normalise_int_type().await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::text());
                t.add_column("ints", types::custom("integer[12]"));
                t.add_column("bools", types::custom("boolean[12]"));
                t.add_column("strings", types::custom("text[12]"));
                t.add_column("floats", types::custom("float[12]"));
                t.set_primary_key(&["id"]);
            });
        })
        .await?;

    let dm = indoc! {r#"
         model Post {
            id       String @id
            ints     Int []
            bools    Boolean []
            strings  String []
            floats   Float []
         }
    "#};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}
