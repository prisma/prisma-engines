use barrel::types;
use enumflags2::BitFlags;
use indoc::indoc;
use introspection_engine_tests::test_api::TestApi;
use introspection_engine_tests::TestResult;
use test_macros::test_connector;

#[test_connector(preview_features("NamedConstraints"))]
async fn introspecting_non_default_pkey_names_works(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Single", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("SomethingCustom", types::primary_constraint(&["id"]));
            });

            migration.create_table("Compound", move |t| {
                t.add_column("a", types::integer().increments(false).nullable(false));
                t.add_column("b", types::integer().increments(false).nullable(false));
                t.add_constraint("SomethingCustomCompound", types::primary_constraint(&["a", "b"]));
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Compound {
          a Int
          b Int
            
          @@id([a, b], map: "SomethingCustomCompound")
        }
              
        model Single {
          id Int @id(map: "SomethingCustom") @default(autoincrement())
        }
    "##};

    let result = &api.introspect().await?;

    api.assert_eq_datamodels(dm, result);
    Ok(())
}
