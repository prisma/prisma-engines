use barrel::types;
use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Mysql))]
async fn empty_preview_features_are_kept(api: &mut TestApi) -> TestResult {
    let schema = indoc! {r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider   = "mysql"
        }
    "#};

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "mysql"
        }
    "#]];

    expectation.assert_eq(&api.re_introspect_config(schema).await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn relation_mode_parameter_is_not_added(api: &mut TestApi) -> TestResult {
    let result = api.re_introspect("").await?;
    assert!(!result.contains(r#"relationMode = "#));

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn multiple_changed_relation_names(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Employee", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Schedule", |t| {
                t.add_column("id", types::primary());
                t.add_column("morningEmployeeId", types::integer().nullable(false));
                t.add_column("eveningEmployeeId", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT morningEmployeeId FOREIGN KEY (morningEmployeeId) REFERENCES `Employee`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );

                t.inject_custom(
                    "CONSTRAINT eveningEmployeeId FOREIGN KEY (eveningEmployeeId) REFERENCES `Employee`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

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
            Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id])
            Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id])

            @@index([eveningEmployeeId], name: "eveningEmployeeId")
            @@index([morningEmployeeId], name: "morningEmployeeId")
        }
    "#};

    let expected = expect![[r#"
        model Employee {
          id                                            Int        @id @default(autoincrement())
          A                                             Schedule[] @relation("EmployeeToSchedule_eveningEmployeeId")
          Schedule_EmployeeToSchedule_morningEmployeeId Schedule[] @relation("EmployeeToSchedule_morningEmployeeId")
        }

        model Schedule {
          id                                            Int      @id @default(autoincrement())
          morningEmployeeId                             Int
          eveningEmployeeId                             Int
          Employee_EmployeeToSchedule_eveningEmployeeId Employee @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id], map: "eveningEmployeeId")
          Employee_EmployeeToSchedule_morningEmployeeId Employee @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id], map: "morningEmployeeId")

          @@index([eveningEmployeeId], map: "eveningEmployeeId")
          @@index([morningEmployeeId], map: "morningEmployeeId")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn mapped_model_and_field_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])

            @@index([c_user_id], name: "user_id")
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
          Custom_User Custom_User @relation(fields: [c_user_id], references: [c_id], map: "user_id")

          @@index([c_user_id], map: "user_id")
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

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn multiple_changed_relation_names_due_to_mapped_models(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false).unique(true));
                t.add_column("user_id2", types::integer().nullable(false).unique(true));

                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
                t.inject_custom(
                    "CONSTRAINT user_id2 FOREIGN KEY (user_id2) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int @id @default(autoincrement())
            user_id          Int  @unique
            user_id2         Int  @unique
            custom_User      Custom_User @relation("CustomRelationName", fields: [user_id], references: [id])
            custom_User2     Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id])
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
          user_id      Int         @unique(map: "user_id")
          user_id2     Int         @unique(map: "user_id2")
          custom_User  Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], map: "user_id")
          custom_User2 Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], map: "user_id2")
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

#[test_connector(tags(Mysql))]
async fn mysql_keeps_renamed_enum_defaults(api: &mut TestApi) -> TestResult {
    let init = formatdoc! {r#"
        CREATE TABLE `A` (
          `id` int NOT NULL AUTO_INCREMENT,
          `val` enum('0','1') NOT NULL DEFAULT '0',
          PRIMARY KEY (`id`)
        );
    "#};

    api.raw_cmd(&init).await;

    let input = indoc! {r#"
        model A {
          id  Int   @id
          val A_val @default(is_false)
        }

        enum A_val {
          is_false @map("0")
          is_true  @map("1")
        }
    "#};

    let expected = expect![[r#"
        model A {
          id  Int   @id @default(autoincrement())
          val A_val @default(is_false)
        }

        enum A_val {
          is_false @map("0")
          is_true  @map("1")
        }
    "#]];

    let result = api.re_introspect_dml(input).await?;
    expected.assert_eq(&result);

    let expected = expect![[r#"
        *** WARNING ***

        These enum values were enriched with `@map` information taken from the previous Prisma schema:
          - Enum: "A_val", value: "is_false"
          - Enum: "A_val", value: "is_true"
    "#]];

    expected.assert_eq(&api.re_introspect_warnings(input).await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn mapped_enum_value_name(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `User` (
            id INT NOT NULL AUTO_INCREMENT,
            color ENUM ('black', 'white') NOT NULL DEFAULT 'black',
            PRIMARY KEY (id)
        );

        CREATE TABLE `Unrelated` (
            id INT NOT NULL AUTO_INCREMENT,
            PRIMARY KEY (id)
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model User {
          id    Int   @id @default(autoincrement())
          color color @default(BLACK)
        }

        enum color {
          BLACK @map("black")
          white
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id    Int   @id @default(autoincrement())
          color color @default(BLACK)
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        enum color {
          BLACK @map("black")
          white
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These enum values were enriched with `@map` information taken from the previous Prisma schema:
          - Enum: "color", value: "BLACK"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}
