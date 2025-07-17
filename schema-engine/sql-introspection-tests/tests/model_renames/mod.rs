use barrel::types;
use indoc::indoc;
use sql_introspection_tests::{TestResult, test_api::*};
use test_macros::test_connector;

#[test_connector(exclude(Postgres, CockroachDb))]
async fn a_table_with_reserved_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("PrismaClient", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("PrismaClient_pkey", types::primary_constraint(vec!["id"]));
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

    let result = api.introspect().await.unwrap();
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn reserved_names_case_sensitivity(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("prismaclient", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("prismaclient_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let dm = indoc! {r#"
        model prismaclient {
          id Int @id @default(autoincrement())
        }
    "#};

    let result = api.introspect().await.unwrap();
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}
