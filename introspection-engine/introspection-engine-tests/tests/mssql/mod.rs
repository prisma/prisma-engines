use indoc::indoc;
use introspection_engine_tests::{test_api::*, TestResult};
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn geometry_should_be_unsupported(api: &TestApi) -> TestResult {
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

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn user_defined_type_aliases_should_map_to_the_system_type(api: &TestApi) -> TestResult {
    let create_type = format!("CREATE TYPE [{}].[Name] FROM [nvarchar](50) NULL", api.schema_name());
    api.database().raw_cmd(&create_type).await?;

    let create_table = format!(
        "CREATE TABLE [{schema_name}].[A] (id int identity primary key, name [{schema_name}].[Name])",
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

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn ms_xml_indexes_are_skipped(api: &TestApi) -> TestResult {
    let create_table = format!(
        "CREATE TABLE [{schema_name}].[xml_test] (id INT IDENTITY PRIMARY KEY, data XML)",
        schema_name = api.schema_name()
    );

    let create_primary = format!(
        "CREATE PRIMARY XML INDEX primaryIndex ON [{schema_name}].[xml_test] (data)",
        schema_name = api.schema_name(),
    );

    let create_secondary = format!(
        "CREATE XML INDEX secondaryIndex ON [{schema_name}].[xml_test] (data) USING XML INDEX primaryIndex FOR PATH",
        schema_name = api.schema_name(),
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;
    api.database().raw_cmd(&create_secondary).await?;

    let dm = indoc! {r#"
        model xml_test {
          id   Int @id @default(autoincrement())
          data String? @db.Xml
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}
