use sql_introspection_tests::{TestResult, test_api::*};

#[test_connector(tags(Mssql))]
async fn multiple_schemas_without_schema_property_are_not_introspected(api: &mut TestApi) -> TestResult {
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
async fn multiple_schemas_w_tables_are_introspected(api: &mut TestApi) -> TestResult {
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

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_tables_are_reintrospected(api: &mut TestApi) -> TestResult {
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

          @@schema("first")
        }

        model B {
          id   Int  @id @default(autoincrement())

          @@schema("second")
        }
    "#};

    let expected = expect![[r#"
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

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_duplicate_table_names_are_introspected(api: &mut TestApi) -> TestResult {
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
          provider = "sqlserver"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first", "second"]
        }

        model first_A {
          id Int @id @default(autoincrement())

          @@map("A")
          @@schema("first")
        }

        model second_A {
          id Int @id @default(autoincrement())

          @@map("A")
          @@schema("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_cross_schema_are_introspected(api: &mut TestApi) -> TestResult {
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
          provider = "sqlserver"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first", "second"]
        }

        model A {
          id Int @id @default(autoincrement())
          B  B[]

          @@schema("first")
        }

        model B {
          id Int  @id @default(autoincrement())
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_cross_schema_are_reintrospected(api: &mut TestApi) -> TestResult {
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

          @@schema("first")
        }

        model B {
          id Int  @id @default(autoincrement())
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("first")
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          B  B[]

          @@schema("first")
        }

        model B {
          id Int  @id @default(autoincrement())
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("second")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_cross_schema_fks_w_duplicate_names_are_introspected(api: &mut TestApi) -> TestResult {
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
          provider = "sqlserver"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first", "second"]
        }

        model first_A {
          id Int        @id @default(autoincrement())
          A  second_A[]

          @@map("A")
          @@schema("first")
        }

        model second_A {
          id Int      @id @default(autoincrement())
          fk Int?
          A  first_A? @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@map("A")
          @@schema("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(
    tags(Mssql),
    preview_features("multiSchema"),
    namespaces("Appointments", "Trips", "core")
)]
async fn schemas_with_varying_case(api: &mut TestApi) -> TestResult {
    for schema in ["Appointments", "Trips", "core"] {
        api.raw_cmd(&format!("CREATE SCHEMA {schema}")).await;
    }

    let setup = formatdoc! {r#"
        CREATE TABLE [Appointments].[Associations] (
            [AppointmentID] BIGINT NOT NULL,
            [AssociatedAppointmentID] BIGINT NOT NULL,
            CONSTRAINT [PK_Associations] PRIMARY KEY CLUSTERED ([AppointmentID],[AssociatedAppointmentID])
        );

        CREATE TABLE [Appointments].[AssociationTypes] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_AssociationTypes] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[billCodes] (
            [Id] BIGINT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_AppointmentBillCode] PRIMARY KEY CLUSTERED ([Id])
        );

        CREATE TABLE [core].[Clusters] (
            [ID] INT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_Clusters] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [core].[Containers] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_Containers] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[Documents] (
            [ID] BIGINT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_AppointmentBOLs] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [core].[Sites] (
            [ID] BIGINT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_Sites] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[statuses] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_AppointmentStatuses] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Trips].[Trips] (
            [ID] BIGINT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_Trips] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Trips].[TripTypes] (
            [ID] INT NOT NULL IDENTITY(1,1),
            CONSTRAINT [PK_TripTypes] PRIMARY KEY CLUSTERED ([ID])
        );
    "#};

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider = "sqlserver"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["Appointments", "Trips", "core"]
        }

        model Associations {
          AppointmentID           BigInt
          AssociatedAppointmentID BigInt

          @@id([AppointmentID, AssociatedAppointmentID], map: "PK_Associations")
          @@schema("Appointments")
        }

        model AssociationTypes {
          ID Int @id(map: "PK_AssociationTypes") @default(autoincrement()) @db.SmallInt

          @@schema("Appointments")
        }

        model billCodes {
          Id BigInt @id(map: "PK_AppointmentBillCode") @default(autoincrement())

          @@schema("Appointments")
        }

        model Clusters {
          ID Int @id(map: "PK_Clusters") @default(autoincrement())

          @@schema("core")
        }

        model Containers {
          ID Int @id(map: "PK_Containers") @default(autoincrement()) @db.SmallInt

          @@schema("core")
        }

        model Documents {
          ID BigInt @id(map: "PK_AppointmentBOLs") @default(autoincrement())

          @@schema("Appointments")
        }

        model Sites {
          ID BigInt @id(map: "PK_Sites") @default(autoincrement())

          @@schema("core")
        }

        model statuses {
          ID Int @id(map: "PK_AppointmentStatuses") @default(autoincrement()) @db.SmallInt

          @@schema("Appointments")
        }

        model Trips {
          ID BigInt @id(map: "PK_Trips") @default(autoincrement())

          @@schema("Trips")
        }

        model TripTypes {
          ID Int @id(map: "PK_TripTypes") @default(autoincrement())

          @@schema("Trips")
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
          provider = "sqlserver"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first", "second"]
        }

        model A {
          id  Int  @id @default(autoincrement())
          val Int? @default(1, map: "test")

          @@schema("first")
        }

        model B {
          id  Int  @id(map: "A_pkey") @default(autoincrement())
          val Int? @default(2, map: "meow")

          @@schema("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
