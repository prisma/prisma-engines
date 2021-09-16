use expect_test::expect;
use introspection_engine_tests::{test_api::*, TestResult};
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn geometry_should_be_unsupported(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [geometry_should_be_unsupported].[A] (
            id INT IDENTITY,
            location GEOGRAPHY,
            CONSTRAINT [A_pkey] PRIMARY KEY (id)
        );
    "#;

    api.raw_cmd(setup).await;

    let result = api.introspect_dml().await?;

    let expected = expect![[r#"
        model A {
          id       Int                       @id @default(autoincrement())
          location Unsupported("geography")?
        }
    "#]];

    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn user_defined_type_aliases_should_map_to_the_system_type(api: &TestApi) -> TestResult {
    let create_type = format!("CREATE TYPE [{}].[Name] FROM [nvarchar](50) NULL", api.schema_name());
    api.database().raw_cmd(&create_type).await?;

    let create_table = format!(
        r#"
        CREATE TABLE [{schema_name}].[A] (
            id INT IDENTITY,
            name [{schema_name}].[Name],
            CONSTRAINT [A_pkey] PRIMARY KEY (id),
        )"#,
        schema_name = api.schema_name()
    );

    api.database().raw_cmd(&create_table).await?;

    let result = api.introspect_dml().await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          name String? @db.NVarChar(50)
        }
    "#]];

    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn ms_xml_indexes_are_skipped(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[xml_test] (
            id INT IDENTITY,
            data XML,

            CONSTRAINT [xml_test_pkey] PRIMARY KEY (id)
        );

        CREATE PRIMARY XML INDEX primaryIndex ON [$schema].[xml_test] (data);
        CREATE XML INDEX secondaryIndex ON [$schema].[xml_test] (data) USING XML INDEX primaryIndex FOR PATH;
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model xml_test {
          id   Int     @id @default(autoincrement())
          data String? @db.Xml
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
