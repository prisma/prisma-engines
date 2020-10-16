use barrel::types;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use pretty_assertions::assert_eq;
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector]
async fn a_table_with_reserved_name(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("StringFilter", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let dm = indoc! {r#"
        /// This model has been renamed to 'RenamedStringFilter' during introspection, because the original name 'StringFilter' is reserved.
        model RenamedStringFilter {
          id Int @id @default(autoincrement())

          @@map("StringFilter")
        }
    "#};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn reserved_names_case_sensitivity(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("query", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let dm = indoc! {r#"
        model query {
          id Int @id @default(autoincrement())
        }
    "#};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}
