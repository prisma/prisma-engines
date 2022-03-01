use barrel::types;
use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mssql))]
async fn multiple_changed_relation_names(api: &TestApi) -> TestResult {
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
async fn multiple_changed_relation_names_due_to_mapped_models(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]))
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
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_constraint("Post_user_id_key", types::unique_constraint(&["user_id"]));
                t.add_constraint("Post_user_id2_key", types::unique_constraint(&["user_id2"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(&["id"]))
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
async fn mapped_model_and_field_name(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]))
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(false));

                t.add_constraint(
                    "Post_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]))
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(&["id"]))
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

    let expected = expect![[
        r#"[{"code":7,"message":"These models were enriched with `@@map` information taken from the previous Prisma schema.","affected":[{"model":"Custom_User"}]},{"code":8,"message":"These fields were enriched with `@map` information taken from the previous Prisma schema.","affected":[{"model":"Post","field":"c_user_id"},{"model":"Custom_User","field":"c_id"}]}]"#
    ]];

    expected.assert_eq(&api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn updated_at(api: &TestApi) {
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
        datasource db {
            provider = "sqlserver"
            url = env("TEST_DATABASE_URL")
        }

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

    api.assert_eq_datamodels(final_dm, &api.re_introspect(input_dm).await.unwrap());
}
