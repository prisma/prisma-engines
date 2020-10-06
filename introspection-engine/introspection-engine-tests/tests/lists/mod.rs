use barrel::types;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector(capabilities("scalar_lists"))]
async fn scalar_list_types(api: &TestApi) -> crate::TestResult {
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

    let dm = indoc! {r#"
        datasource pg {
            provider = "postgres"
            url = "postgresql://localhost:5432"
        }

        model Post {
            id      Int @id @default(autoincrement())
            ints     Int []
            bools    Boolean []
            strings  String []
            floats   Float []
            }
    "#};

    let result = format!(
        r#"
        datasource pg {{
            provider = "postgres"
            url = "postgresql://localhost:5432"
        }}

        {}
    "#,
        api.introspect().await?
    );

    assert_eq_datamodels!(dm, &result);

    Ok(())
}
