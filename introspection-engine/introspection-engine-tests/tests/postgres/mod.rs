use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector(tags("postgres"))]
async fn sequences_should_work(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.inject_custom("CREATE SEQUENCE \"first_Sequence\"");
            migration.inject_custom("CREATE SEQUENCE \"second_sequence\"");
            migration.inject_custom("CREATE SEQUENCE \"third_Sequence\"");

            migration.create_table("Test", move |t| {
                t.inject_custom("id Integer Primary Key");
                t.inject_custom("serial  Serial");
                t.inject_custom("first   BigInt Default nextval('\"first_Sequence\"'::regclass)");
                t.inject_custom("second  BigInt Default nextval('\"second_sequence\"')");
                t.inject_custom("third  BigInt Default nextval('third_Sequence'::text)");
            });
        })
        .await?;

    let dm = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
        }
       
        datasource postgres {
            provider        = "postgres"
            url             = "postgres://localhost/test"
        }

        model Test {
          id     Int @id
          serial Int @default(autoincrement())
          first  Int @default(autoincrement())
          second Int @default(autoincrement())
          third  Int @default(autoincrement())
        }
    "#}
    .to_string();

    let result = api.re_introspect(&dm).await?;

    println!("EXPECTATION: \n {:#}", dm);
    println!("RESULT: \n {:#}", result);

    assert_eq_datamodels!(&result, &dm);
    Ok(())
}
