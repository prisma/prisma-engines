use barrel::types;
use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Mssql))]
async fn multiple_changed_relation_names(api: &mut TestApi) -> TestResult {
    let setup = format!(
        r#"
        CREATE TABLE [{schema}].[Employee] (
            id INTEGER IDENTITY,

            CONSTRAINT [Employee_pkey] PRIMARY KEY ("id")
        );


        CREATE TABLE [{schema}].[Schedule] (
            id INTEGER IDENTITY,
            [morningEmployeeId] INTEGER,
            [eveningEmployeeId] INTEGER,

            CONSTRAINT [morning_fkey] FOREIGN KEY ([morningEmployeeId]) REFERENCES [{schema}].[Employee] ("id"),
            CONSTRAINT [evening_fkey] FOREIGN KEY ([eveningEmployeeId]) REFERENCES [{schema}].[Employee] ("id"),
            CONSTRAINT [Schedule_pkey] PRIMARY KEY ("id")
        );

        CREATE TABLE [{schema}].[Unrelated] (
            id INTEGER IDENTITY,

            CONSTRAINT [Unrelated_pkey] PRIMARY KEY ("id")
        );
        "#,
        schema = api.schema_name()
    );

    api.raw_cmd(&setup).await;

    let input_dm = indoc! {r#"
        model Employee {
            id                                            Int         @id @default(autoincrement())
            A                                             Schedule[]  @relation("EmployeeToSchedule_eveningEmployeeId")
            Schedule_EmployeeToSchedule_morningEmployeeId Schedule[]  @relation("EmployeeToSchedule_morningEmployeeId")
        }

        model Schedule {
            id                                            Int         @id @default(autoincrement())
            morningEmployeeId                             Int
            eveningEmployeeId                             Int
            Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id], onUpdate: NoAction)
            Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id], onUpdate: NoAction)
        }
    "#};

    let expected = expect![[r#"
        model Employee {
          id                                            Int        @id @default(autoincrement())
          A                                             Schedule[] @relation("EmployeeToSchedule_eveningEmployeeId")
          Schedule_EmployeeToSchedule_morningEmployeeId Schedule[] @relation("EmployeeToSchedule_morningEmployeeId")
        }

        model Schedule {
          id                                            Int       @id @default(autoincrement())
          morningEmployeeId                             Int?
          eveningEmployeeId                             Int?
          Employee_EmployeeToSchedule_eveningEmployeeId Employee? @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "evening_fkey")
          Employee_EmployeeToSchedule_morningEmployeeId Employee? @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "morning_fkey")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn multiple_changed_relation_names_due_to_mapped_models(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]))
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("user_id2", types::integer().nullable(false));

                t.add_constraint(
                    "post_userid_fk",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
                t.add_constraint(
                    "post_userid2_fk",
                    types::foreign_constraint(&["user_id2"], "User", &["id"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
                t.add_constraint("Post_user_id_key", types::unique_constraint(["user_id"]));
                t.add_constraint("Post_user_id2_key", types::unique_constraint(["user_id2"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(["id"]))
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int @id @default(autoincrement())
            user_id          Int  @unique
            user_id2         Int  @unique
            custom_User      Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], onUpdate: NoAction)
            custom_User2     Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], onUpdate: NoAction)
        }

        model Custom_User {
            id               Int @id @default(autoincrement())
            custom_Post      Post? @relation("CustomRelationName")
            custom_Post2     Post? @relation("AnotherCustomRelationName")

            @@map("User")
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id           Int         @id @default(autoincrement())
          user_id      Int         @unique
          user_id2     Int         @unique
          custom_User  Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], onUpdate: NoAction, map: "post_userid_fk")
          custom_User2 Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], onUpdate: NoAction, map: "post_userid2_fk")
        }

        model Custom_User {
          id           Int   @id @default(autoincrement())
          custom_Post  Post? @relation("CustomRelationName")
          custom_Post2 Post? @relation("AnotherCustomRelationName")

          @@map("User")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn mapped_model_and_field_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]))
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(false));

                t.add_constraint(
                    "Post_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]))
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(["id"]))
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
        }

        model Custom_User {
            c_id             Int         @id @default(autoincrement()) @map("id")
            Post             Post[]

            @@map(name: "User")
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id          Int         @id @default(autoincrement())
          c_user_id   Int         @map("user_id")
          Custom_User Custom_User @relation(fields: [c_user_id], references: [c_id], onUpdate: NoAction, map: "Post_fkey")
        }

        model Custom_User {
          c_id Int    @id @default(autoincrement()) @map("id")
          Post Post[]

          @@map("User")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    let expected = expect![[r#"
        *** WARNING ***

        These fields were enriched with `@map` information taken from the previous Prisma schema:
          - Model: "Post", field: "c_user_id"
          - Model: "Custom_User", field: "c_id"

        These models were enriched with `@@map` information taken from the previous Prisma schema:
          - "Custom_User"
    "#]];

    expected.assert_eq(&api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn updated_at(api: &mut TestApi) {
    let setup = format!(
        r#"
        CREATE TABLE [{schema_name}].[User] (
            id INTEGER,
            [lastupdated] DATETIME,
            [lastupdated2] DATETIME2,

            CONSTRAINT [User_pkey] PRIMARY KEY ([id])
        );

        CREATE TABLE [{schema_name}].[Unrelated] (
            id INTEGER IDENTITY,

            CONSTRAINT [Unrelated_pkey] PRIMARY KEY ([id])
        );
        "#,
        schema_name = api.schema_name()
    );

    api.raw_cmd(&setup).await;

    let input_dm = indoc! {r#"
        model User {
            id           Int    @id
            lastupdated  DateTime? @updatedAt
            lastupdated2 DateTime? @db.DateTime @updatedAt
        }
    "#};

    let final_dm = indoc! {r#"
        model User {
            id           Int    @id
            lastupdated  DateTime? @updatedAt @db.DateTime
            lastupdated2 DateTime? @updatedAt
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }
    "#};

    let result = api.re_introspect(input_dm).await.unwrap();
    api.assert_eq_datamodels(final_dm, &result);
}

#[test_connector(tags(Mssql))]
async fn re_introspecting_custom_compound_id_names(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE [User] (
            first INT NOT NULL,
            last INT NOT NULL,
            CONSTRAINT [User.something@invalid-and/weird] PRIMARY KEY (first, last)
        );

        CREATE TABLE [User2] (
            first INT NOT NULL,
            last INT NOT NULL,
            CONSTRAINT [User2_pkey] PRIMARY KEY (first, last)
        );

        CREATE TABLE [Unrelated] (
            id INT IDENTITY,
            CONSTRAINT [Unrelated_pkey] PRIMARY KEY (id)
        )
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
         model User {
           first  Int
           last   Int

           @@id([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }

         model User2 {
           first  Int
           last   Int

           @@id([first, last], name: "compound")
         }
     "#};

    let expectation = expect![[r#"
         model User {
           first Int
           last  Int

           @@id([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }

         model User2 {
           first Int
           last  Int

           @@id([first, last], name: "compound")
         }

         model Unrelated {
           id Int @id @default(autoincrement())
         }
     "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expected = expect![[r#"
        *** WARNING ***

        These models were enriched with custom compound id names taken from the previous Prisma schema:
          - "User"
          - "User2"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn re_introspecting_custom_compound_unique_names(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE [User] (
            id INT IDENTITY,
            first INT NOT NULL,
            last INT NOT NULL,
            CONSTRAINT [User.something@invalid-and/weird] UNIQUE (first, last),
            CONSTRAINT [User_pkey] PRIMARY KEY (id)
        );

        CREATE TABLE [Unrelated] (
            id INT IDENTITY,
            CONSTRAINT [Unrelated_pkey] PRIMARY KEY (id)
        )
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
         model User {
           id    Int @id @default(autoincrement())
           first Int
           last  Int

           @@unique([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }
     "#};

    let expectation = expect![[r#"
         model User {
           id    Int @id @default(autoincrement())
           first Int
           last  Int

           @@unique([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }

         model Unrelated {
           id Int @id @default(autoincrement())
         }
     "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn direct_url(api: &mut TestApi) {
    let setup = format!(
        r#"
        CREATE TABLE [{schema_name}].[User] (
            id INTEGER,
            [lastupdated] DATETIME,
            [lastupdated2] DATETIME2,

            CONSTRAINT [User_pkey] PRIMARY KEY ([id])
        );

        CREATE TABLE [{schema_name}].[Unrelated] (
            id INTEGER IDENTITY,

            CONSTRAINT [Unrelated_pkey] PRIMARY KEY ([id])
        );
        "#,
        schema_name = api.schema_name()
    );

    api.raw_cmd(&setup).await;

    let input_dm = indoc! {r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider  = "sqlserver"
          url       = "bad url"
          directUrl = "dummy-url"
        }

        model User {
          id           Int       @id
          lastupdated  DateTime? @updatedAt
          lastupdated2 DateTime? @db.DateTime @updatedAt
        }
    "#};

    let final_dm = indoc! {r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider  = "sqlserver"
          url       = "bad url"
          directUrl = "dummy-url"
        }

        model User {
          id           Int       @id
          lastupdated  DateTime? @updatedAt @db.DateTime
          lastupdated2 DateTime? @updatedAt
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#};

    pretty_assertions::assert_eq!(final_dm, &api.re_introspect_config(input_dm).await.unwrap());
}
