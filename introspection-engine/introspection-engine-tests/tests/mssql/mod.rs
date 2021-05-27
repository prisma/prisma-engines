use barrel::types;
use indoc::indoc;
use introspection_engine_tests::{test_api::*, TestResult};
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn geometry_should_be_unsupported(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("A", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("A_pkey", types::primary_constraint(&["id"]));
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

#[test_connector(tags(Mssql))]
async fn user_defined_type_aliases_should_map_to_the_system_type(api: &TestApi) -> TestResult {
    let create_type = format!("CREATE TYPE [{}].[Name] FROM [nvarchar](50) NULL", api.schema_name());
    api.database().raw_cmd(&create_type).await?;

    let create_table = format!(
        "CREATE TABLE [{schema_name}].[A] (\
            id int identity, \
            name [{schema_name}].[Name],\
            CONSTRAINT [A_pkey] PRIMARY KEY ([id]))",
        schema_name = api.schema_name()
    );

    api.database().raw_cmd(&create_table).await?;

    let dm = indoc! {r#"
        model A {
          id       Int @id @default(autoincrement())
          name     String? @db.NVarChar(50)
        }
    "#};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}
