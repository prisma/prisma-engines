use introspection_engine_tests::{test_api::*, TestResult};

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_tables_are_introspected(api: &TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    let setup = format!("CREATE SCHEMA {schema_name}");
    api.database().raw_cmd(&setup).await?;

    let setup = format!("CREATE SCHEMA {other_name}");
    api.database().raw_cmd(&setup).await?;

    let setup = format!("CREATE SCHEMA third");
    api.database().raw_cmd(&setup).await?;

    let setup = formatdoc!(
        r#"
        CREATE TABLE [{schema_name}].[A] (
            id INT IDENTITY,
            data INT,
            CONSTRAINT [A_pkey] PRIMARY KEY (id)
        );
    "#
    );
    api.database().raw_cmd(&setup).await?;

    let setup = formatdoc!(
        r#"
        CREATE TABLE [{other_name}].[B] (
            id INT IDENTITY,
            data INT,
            CONSTRAINT [B_pkey] PRIMARY KEY (id)
        );
    "#
    );
    api.database().raw_cmd(&setup).await?;

    let setup = formatdoc!(
        r#"
        CREATE TABLE [third].[C] (
            id INT IDENTITY,
            data INT,
            CONSTRAINT [C_pkey] PRIMARY KEY (id)
        );
    "#
    );
    api.database().raw_cmd(&setup).await?;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider = "sqlserver"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first", "second"]
        }

        model A {
          id   Int  @id @default(autoincrement())
          data Int?

          @@schema("first")
        }

        model B {
          id   Int  @id @default(autoincrement())
          data Int?

          @@schema("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
