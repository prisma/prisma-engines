use sql_introspection_tests::{test_api::*, TestResult};

#[test_connector(tags(Mssql))]
async fn multiple_namespaces_without_schema_property_are_not_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let other_name = "second";

    let setup = formatdoc! {r#"
        CREATE TABLE [{schema_name}].[A] (id INT IDENTITY, data INT, CONSTRAINT A_pkey PRIMARY KEY (id));
        CREATE INDEX [A_idx] ON [{schema_name}].[A] ([data]);
    "#};

    api.raw_cmd(&setup).await;
    api.raw_cmd(&format!("CREATE SCHEMA {other_name}")).await;

    let setup = formatdoc! {r#"
        CREATE TABLE [{other_name}].[B] (id INT IDENTITY PRIMARY KEY, data INT);
    "#};

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "sqlserver"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int  @id @default(autoincrement())
          data Int?

          @@index([data], map: "A_idx")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_namespaces_w_tables_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    let setup = format!("CREATE SCHEMA {schema_name}");
    api.database().raw_cmd(&setup).await?;

    let setup = format!("CREATE SCHEMA {other_name}");
    api.database().raw_cmd(&setup).await?;

    let setup = "CREATE SCHEMA third".to_string();
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
          provider   = "sqlserver"
          url        = "env(TEST_DATABASE_URL)"
          namespaces = ["first", "second"]
        }

        model A {
          id   Int  @id @default(autoincrement())
          data Int?

          @@namespace("first")
        }

        model B {
          id   Int  @id @default(autoincrement())
          data Int?

          @@namespace("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_namespaces_w_tables_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    let setup = format!("CREATE SCHEMA {schema_name}");
    api.database().raw_cmd(&setup).await?;

    let setup = format!("CREATE SCHEMA {other_name}");
    api.database().raw_cmd(&setup).await?;

    let setup = "CREATE SCHEMA third".to_string();
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

    let input = indoc! {r#"
        model A {
          id   Int  @id @default(autoincrement())

          @@namespace("first")
        }

        model B {
          id   Int  @id @default(autoincrement())

          @@namespace("second")
        }
    "#};

    let expected = expect![[r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Int?

          @@namespace("first")
        }

        model B {
          id   Int  @id @default(autoincrement())
          data Int?

          @@namespace("second")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_namespaces_w_duplicate_table_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    api.raw_cmd(&format!("CREATE SCHEMA {schema_name}")).await;
    api.raw_cmd(&format!("CREATE SCHEMA {other_name}")).await;

    let setup = formatdoc! {r#"
        CREATE TABLE [{schema_name}].[A] (id INT IDENTITY, CONSTRAINT A_pkey PRIMARY KEY (id));
        CREATE TABLE [{other_name}].[A] (id INT IDENTITY, CONSTRAINT A_pkey PRIMARY KEY (id));
    "#};

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider   = "sqlserver"
          url        = "env(TEST_DATABASE_URL)"
          namespaces = ["first", "second"]
        }

        model first_A {
          id Int @id @default(autoincrement())

          @@map("A")
          @@namespace("first")
        }

        model second_A {
          id Int @id @default(autoincrement())

          @@map("A")
          @@namespace("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_namespaces_w_cross_schema_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    api.raw_cmd(&format!("CREATE SCHEMA {schema_name}")).await;
    api.raw_cmd(&format!("CREATE SCHEMA {other_name}")).await;

    let setup = formatdoc! {r#"
        CREATE TABLE [{schema_name}].[A] (
            [id] INT IDENTITY,
            CONSTRAINT A_pkey PRIMARY KEY ([id])
        );

        CREATE TABLE [{other_name}].[B] (
            [id] INT IDENTITY,
            [fk] INT,
            CONSTRAINT B_pkey PRIMARY KEY ([id]),
            CONSTRAINT B_fk_fkey FOREIGN KEY (fk) REFERENCES [{schema_name}].[A]([id]))
    "#};

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider   = "sqlserver"
          url        = "env(TEST_DATABASE_URL)"
          namespaces = ["first", "second"]
        }

        model A {
          id Int @id @default(autoincrement())
          B  B[]

          @@namespace("first")
        }

        model B {
          id Int  @id @default(autoincrement())
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@namespace("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_namespaces_w_cross_schema_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    api.raw_cmd(&format!("CREATE SCHEMA {schema_name}")).await;
    api.raw_cmd(&format!("CREATE SCHEMA {other_name}")).await;

    let setup = formatdoc! {r#"
        CREATE TABLE [{schema_name}].[A] (
            [id] INT IDENTITY,
            CONSTRAINT A_pkey PRIMARY KEY ([id])
        );

        CREATE TABLE [{other_name}].[B] (
            [id] INT IDENTITY,
            [fk] INT,
            CONSTRAINT B_pkey PRIMARY KEY ([id]),
            CONSTRAINT B_fk_fkey FOREIGN KEY (fk) REFERENCES [{schema_name}].[A]([id]))
    "#};

    api.raw_cmd(&setup).await;

    let input = indoc! {r#"
        model A {
          id Int @id @default(autoincrement())
          B  B[]

          @@namespace("first")
        }

        model B {
          id Int  @id @default(autoincrement())
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@namespace("first")
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          B  B[]

          @@namespace("first")
        }

        model B {
          id Int  @id @default(autoincrement())
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@namespace("second")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_namespaces_w_cross_schema_fks_w_duplicate_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    api.raw_cmd(&format!("CREATE SCHEMA {schema_name}")).await;
    api.raw_cmd(&format!("CREATE SCHEMA {other_name}")).await;

    let setup = formatdoc! {r#"
        CREATE TABLE [{schema_name}].[A] (
            [id] INT IDENTITY,
            CONSTRAINT A_pkey PRIMARY KEY ([id])
        );

        CREATE TABLE [{other_name}].[A] (
            [id] INT IDENTITY,
            [fk] INT,
            CONSTRAINT A_pkey PRIMARY KEY ([id]),
            CONSTRAINT A_fk_fkey FOREIGN KEY (fk) REFERENCES [{schema_name}].[A]([id]))
    "#};

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider   = "sqlserver"
          url        = "env(TEST_DATABASE_URL)"
          namespaces = ["first", "second"]
        }

        model first_A {
          id Int        @id @default(autoincrement())
          A  second_A[]

          @@map("A")
          @@namespace("first")
        }

        model second_A {
          id Int      @id @default(autoincrement())
          fk Int?
          A  first_A? @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@map("A")
          @@namespace("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn defaults_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";

    api.raw_cmd(&format!("CREATE SCHEMA {schema_name}")).await;
    api.raw_cmd(&format!("CREATE SCHEMA {other_name}")).await;

    let setup = formatdoc! {r#"
        CREATE TABLE [{schema_name}].[A] (
            [id] INT IDENTITY,
            [val] INT CONSTRAINT [test] DEFAULT 1,
            CONSTRAINT A_pkey PRIMARY KEY (id)
        );

        CREATE TABLE [{other_name}].[B] (
            [id] INT IDENTITY,
            [val] INT CONSTRAINT [meow] DEFAULT 2,
            CONSTRAINT A_pkey PRIMARY KEY (id)
        );
    "#};

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider   = "sqlserver"
          url        = "env(TEST_DATABASE_URL)"
          namespaces = ["first", "second"]
        }

        model A {
          id  Int  @id @default(autoincrement())
          val Int? @default(1, map: "test")

          @@namespace("first")
        }

        model B {
          id  Int  @id(map: "A_pkey") @default(autoincrement())
          val Int? @default(2, map: "meow")

          @@namespace("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
