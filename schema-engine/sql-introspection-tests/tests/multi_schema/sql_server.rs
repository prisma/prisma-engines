use sql_introspection_tests::{test_api::*, TestResult};

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
            [AssociationTypeID] SMALLINT NOT NULL,
            [Priority] SMALLINT NOT NULL,
            CONSTRAINT [PK_Associations] PRIMARY KEY CLUSTERED ([AppointmentID],[AssociatedAppointmentID])
        );

        CREATE TABLE [Appointments].[AssociationTypes] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(50) NOT NULL,
            CONSTRAINT [PK_AssociationTypes] PRIMARY KEY CLUSTERED ([ID])
        );

        -- CreateTable
        CREATE TABLE [Appointments].[billCodes] (
            [Id] BIGINT NOT NULL IDENTITY(1,1),
            [HayesId] NVARCHAR(128),
            [Article] NVARCHAR(128),
            [Description] NVARCHAR(128),
            [SP_ProjectType] NVARCHAR(20),
            [LTLallowed] BIT NOT NULL CONSTRAINT [DF_AppointmentBillCode_LTLallowed] DEFAULT 0,
            [FTLallowed] BIT NOT NULL CONSTRAINT [DF_AppointmentBillCode_FTLallowed] DEFAULT 0,
            [CustomerID] SMALLINT,
            CONSTRAINT [PK_AppointmentBillCode] PRIMARY KEY CLUSTERED ([Id])
        );

        CREATE TABLE [Appointments].[ChargeTypes] (
            [ID] TINYINT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(50) NOT NULL,
            [BOL_Description] NVARCHAR(128),
            [_rv] timestamp NOT NULL,
            CONSTRAINT [PK_ChargeTypes] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [core].[Clusters] (
            [ID] INT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(128) NOT NULL,
            [TenantID] INT NOT NULL,
            CONSTRAINT [PK_Clusters] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [core].[Containers] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            [TenantID] INT NOT NULL,
            [Description] NVARCHAR(100) NOT NULL,
            [Width] INT NOT NULL,
            [Length] INT NOT NULL,
            [Units] NVARCHAR(10) NOT NULL,
            [Area] INT,
            CONSTRAINT [PK_Containers] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[Documents] (
            [ID] BIGINT NOT NULL IDENTITY(1,1),
            [DocumentTypeID] SMALLINT NOT NULL CONSTRAINT [DF_AppointmentBOLs_DocumentTypeID] DEFAULT 1,
            [ApptID] BIGINT NOT NULL,
            [Name] NVARCHAR(256) NOT NULL,
            [Path] NVARCHAR(max),
            [CreationDate] SMALLDATETIME NOT NULL,
            [CreatedByUserID] INT NOT NULL,
            [TemplateVersion] INT,
            [_rv] timestamp NOT NULL,
            [WebPath] NVARCHAR(max),
            [SsrsUrl] NVARCHAR(max),
            [FilePath] NVARCHAR(max) NOT NULL,
            CONSTRAINT [PK_AppointmentBOLs] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[DocumentTypes] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(128) NOT NULL,
            [Report] NVARCHAR(128),
            [_rv] timestamp NOT NULL,
            [Inbound] BIT,
            [Outbound] BIT,
            [Limit] INT,
            CONSTRAINT [PK_DocumentTypes_1] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[ItineraryTypes] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(50) NOT NULL,
            CONSTRAINT [PK_ItineraryTypes] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[Queues] (
            [Id] BIGINT NOT NULL IDENTITY(1,1),
            [SiteID] BIGINT,
            [QueueTypeID] TINYINT,
            [Location] NVARCHAR(128),
            [Name] NVARCHAR(128),
            [Direction] VARCHAR(6),
            [Active] BIT,
            [_rv] timestamp NOT NULL,
            [Export] BIT,
            CONSTRAINT [PK_Queues] PRIMARY KEY CLUSTERED ([Id]),
            CONSTRAINT [IX_Locations] UNIQUE NONCLUSTERED ([Location])
        );

        CREATE TABLE [Appointments].[QueueTypes] (
            [ID] TINYINT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(50) NOT NULL,
            [Description] NVARCHAR(128),
            [_rv] timestamp NOT NULL,
            CONSTRAINT [PK_QueueTypes] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[Settings] (
            [ID] INT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(255) NOT NULL,
            [Value] NVARCHAR(max),
            [Notes] NVARCHAR(max),
            [_rv] timestamp NOT NULL,
            CONSTRAINT [PK_Settings] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[Settings_Overrides] (
            [ID] INT NOT NULL IDENTITY(1,1),
            [SettingID] INT NOT NULL,
            [Active] BIT NOT NULL,
            [TenantID] SMALLINT,
            [SiteID] SMALLINT,
            [CustomerID] SMALLINT,
            [QueueID] SMALLINT,
            [Direction] NVARCHAR(10),
            [LiveLoad] BIT,
            [PreviousApptStatusID] SMALLINT,
            [ApptStatusID] SMALLINT,
            [Value] NVARCHAR(max) NOT NULL,
            CONSTRAINT [PK_Settings_Override] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [core].[Sites] (
            [ID] BIGINT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(128),
            [StreetAddress] NVARCHAR(128),
            [City] NVARCHAR(128),
            [State] NVARCHAR(128),
            [PostalCode] NVARCHAR(15),
            [WMShipPoint] NVARCHAR(20),
            [LTLout] BIT NOT NULL CONSTRAINT [DF_Sites_LTLout] DEFAULT 0,
            [_rv] timestamp NOT NULL,
            [SiteName] NVARCHAR(128),
            [TenantID] INT,
            [ParentID] INT,
            [Code] NVARCHAR(10),
            [Active] BIT,
            [WhseNumber] NVARCHAR(50),
            [TimeZone] NVARCHAR(50),
            [TZ] NVARCHAR(10),
            [ClusterID] INT,
            CONSTRAINT [PK_Sites] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Appointments].[statuses] (
            [ID] SMALLINT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(50),
            [Status] INT,
            [Description] NVARCHAR(51) NOT NULL,
            [DockDoorRequired] BIT NOT NULL CONSTRAINT [DF_AppointmentStatuses_DockDoorRequred] DEFAULT 0,
            [DockTypeID] SMALLINT,
            CONSTRAINT [PK_AppointmentStatuses] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Trips].[Trips] (
            [ID] BIGINT NOT NULL IDENTITY(1,1),
            [TripTypeID] INT NOT NULL,
            [TripName] NVARCHAR(120) NOT NULL,
            CONSTRAINT [PK_Trips] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE TABLE [Trips].[TripTypes] (
            [ID] INT NOT NULL IDENTITY(1,1),
            [Name] NVARCHAR(128) NOT NULL,
            [TenantID] INT NOT NULL,
            CONSTRAINT [PK_TripTypes] PRIMARY KEY CLUSTERED ([ID])
        );

        CREATE NONCLUSTERED INDEX [<Name of Missing Index, sysname,>] ON [Appointments].[Documents]([DocumentTypeID], [ApptID]);

        ALTER TABLE [core].[Sites] ADD CONSTRAINT [FK_Sites_Clusters] FOREIGN KEY ([ClusterID]) REFERENCES [core].[Clusters]([ID]) ON DELETE SET NULL ON UPDATE NO ACTION;

        ALTER TABLE [Trips].[Trips] ADD CONSTRAINT [FK_Trips_TripTypes] FOREIGN KEY ([TripTypeID]) REFERENCES [Trips].[TripTypes]([ID]) ON DELETE NO ACTION ON UPDATE NO ACTION;
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
          AssociationTypeID       Int    @db.SmallInt
          Priority                Int    @db.SmallInt

          @@id([AppointmentID, AssociatedAppointmentID], map: "PK_Associations")
          @@schema("Appointments")
        }

        model AssociationTypes {
          ID   Int    @id(map: "PK_AssociationTypes") @default(autoincrement()) @db.SmallInt
          Name String @db.NVarChar(50)

          @@schema("Appointments")
        }

        model billCodes {
          Id             BigInt  @id(map: "PK_AppointmentBillCode") @default(autoincrement())
          HayesId        String? @db.NVarChar(128)
          Article        String? @db.NVarChar(128)
          Description    String? @db.NVarChar(128)
          SP_ProjectType String? @db.NVarChar(20)
          LTLallowed     Boolean @default(false, map: "DF_AppointmentBillCode_LTLallowed")
          FTLallowed     Boolean @default(false, map: "DF_AppointmentBillCode_FTLallowed")
          CustomerID     Int?    @db.SmallInt

          @@schema("Appointments")
        }

        model ChargeTypes {
          ID              Int                      @id(map: "PK_ChargeTypes") @default(autoincrement()) @db.TinyInt
          Name            String                   @db.NVarChar(50)
          BOL_Description String?                  @db.NVarChar(128)
          rv              Unsupported("timestamp") @map("_rv")

          @@schema("Appointments")
        }

        model Clusters {
          ID       Int     @id(map: "PK_Clusters") @default(autoincrement())
          Name     String  @db.NVarChar(128)
          TenantID Int
          Sites    Sites[]

          @@schema("core")
        }

        model Containers {
          ID          Int    @id(map: "PK_Containers") @default(autoincrement()) @db.SmallInt
          TenantID    Int
          Description String @db.NVarChar(100)
          Width       Int
          Length      Int
          Units       String @db.NVarChar(10)
          Area        Int?

          @@schema("core")
        }

        model Documents {
          ID              BigInt                   @id(map: "PK_AppointmentBOLs") @default(autoincrement())
          DocumentTypeID  Int                      @default(1, map: "DF_AppointmentBOLs_DocumentTypeID") @db.SmallInt
          ApptID          BigInt
          Name            String                   @db.NVarChar(256)
          Path            String?                  @db.NVarChar(Max)
          CreationDate    DateTime                 @db.SmallDateTime
          CreatedByUserID Int
          TemplateVersion Int?
          rv              Unsupported("timestamp") @map("_rv")
          WebPath         String?                  @db.NVarChar(Max)
          SsrsUrl         String?                  @db.NVarChar(Max)
          FilePath        String                   @db.NVarChar(Max)

          @@index([DocumentTypeID, ApptID], map: "<Name of Missing Index, sysname,>")
          @@schema("Appointments")
        }

        model DocumentTypes {
          ID       Int                      @id(map: "PK_DocumentTypes_1") @default(autoincrement()) @db.SmallInt
          Name     String                   @db.NVarChar(128)
          Report   String?                  @db.NVarChar(128)
          rv       Unsupported("timestamp") @map("_rv")
          Inbound  Boolean?
          Outbound Boolean?
          Limit    Int?

          @@schema("Appointments")
        }

        model ItineraryTypes {
          ID   Int    @id(map: "PK_ItineraryTypes") @default(autoincrement()) @db.SmallInt
          Name String @db.NVarChar(50)

          @@schema("Appointments")
        }

        model Queues {
          Id          BigInt                   @id(map: "PK_Queues") @default(autoincrement())
          SiteID      BigInt?
          QueueTypeID Int?                     @db.TinyInt
          Location    String?                  @unique(map: "IX_Locations") @db.NVarChar(128)
          Name        String?                  @db.NVarChar(128)
          Direction   String?                  @db.VarChar(6)
          Active      Boolean?
          rv          Unsupported("timestamp") @map("_rv")
          Export      Boolean?

          @@schema("Appointments")
        }

        model QueueTypes {
          ID          Int                      @id(map: "PK_QueueTypes") @default(autoincrement()) @db.TinyInt
          Name        String                   @db.NVarChar(50)
          Description String?                  @db.NVarChar(128)
          rv          Unsupported("timestamp") @map("_rv")

          @@schema("Appointments")
        }

        model Settings {
          ID    Int                      @id(map: "PK_Settings") @default(autoincrement())
          Name  String                   @db.NVarChar(255)
          Value String?                  @db.NVarChar(Max)
          Notes String?                  @db.NVarChar(Max)
          rv    Unsupported("timestamp") @map("_rv")

          @@schema("Appointments")
        }

        model Settings_Overrides {
          ID                   Int      @id(map: "PK_Settings_Override") @default(autoincrement())
          SettingID            Int
          Active               Boolean
          TenantID             Int?     @db.SmallInt
          SiteID               Int?     @db.SmallInt
          CustomerID           Int?     @db.SmallInt
          QueueID              Int?     @db.SmallInt
          Direction            String?  @db.NVarChar(10)
          LiveLoad             Boolean?
          PreviousApptStatusID Int?     @db.SmallInt
          ApptStatusID         Int?     @db.SmallInt
          Value                String   @db.NVarChar(Max)

          @@schema("Appointments")
        }

        model Sites {
          ID            BigInt                   @id(map: "PK_Sites") @default(autoincrement())
          Name          String?                  @db.NVarChar(128)
          StreetAddress String?                  @db.NVarChar(128)
          City          String?                  @db.NVarChar(128)
          State         String?                  @db.NVarChar(128)
          PostalCode    String?                  @db.NVarChar(15)
          WMShipPoint   String?                  @db.NVarChar(20)
          LTLout        Boolean                  @default(false, map: "DF_Sites_LTLout")
          rv            Unsupported("timestamp") @map("_rv")
          SiteName      String?                  @db.NVarChar(128)
          TenantID      Int?
          ParentID      Int?
          Code          String?                  @db.NVarChar(10)
          Active        Boolean?
          WhseNumber    String?                  @db.NVarChar(50)
          TimeZone      String?                  @db.NVarChar(50)
          TZ            String?                  @db.NVarChar(10)
          ClusterID     Int?
          Clusters      Clusters?                @relation(fields: [ClusterID], references: [ID], onUpdate: NoAction, map: "FK_Sites_Clusters")

          @@schema("core")
        }

        model statuses {
          ID               Int     @id(map: "PK_AppointmentStatuses") @default(autoincrement()) @db.SmallInt
          Name             String? @db.NVarChar(50)
          Status           Int?
          Description      String  @db.NVarChar(51)
          DockDoorRequired Boolean @default(false, map: "DF_AppointmentStatuses_DockDoorRequred")
          DockTypeID       Int?    @db.SmallInt

          @@schema("Appointments")
        }

        model Trips {
          ID         BigInt    @id(map: "PK_Trips") @default(autoincrement())
          TripTypeID Int
          TripName   String    @db.NVarChar(120)
          TripTypes  TripTypes @relation(fields: [TripTypeID], references: [ID], onUpdate: NoAction, map: "FK_Trips_TripTypes")

          @@schema("Trips")
        }

        model TripTypes {
          ID       Int     @id(map: "PK_TripTypes") @default(autoincrement())
          Name     String  @db.NVarChar(128)
          TenantID Int
          Trips    Trips[]

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
