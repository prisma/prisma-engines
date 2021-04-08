use indoc::indoc;
use introspection_engine_tests::test_api::*;
use test_macros::test_each_connector;

#[test_each_connector(tags("mssql"))]
async fn geometry_should_be_unsupported(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("A", move |t| {
                t.inject_custom("id int identity primary key");
                t.inject_custom("location geography");
            });
        })
        .await?;

    let result = api.introspect().await?;

    let dm = indoc! {r#"
        model A {
          id       Int @id @default(autoincrement())
          location Unsupported("geography")?
        }
    "#};

    api.assert_eq_datamodels(&dm, &result);
    Ok(())
}
