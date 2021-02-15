use barrel::types;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use test_macros::test_each_connector;

#[test_each_connector]
async fn a_table_with_reserved_name(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("PrismaClient", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let dm = indoc! {r#"
        /// This model has been renamed to 'RenamedPrismaClient' during introspection, because the original name 'PrismaClient' is reserved.
        model RenamedPrismaClient {
          id Int @id @default(autoincrement())

          @@map("PrismaClient")
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn reserved_names_case_sensitivity(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("prismaclient", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let dm = indoc! {r#"
        model prismaclient {
          id Int @id @default(autoincrement())
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}
