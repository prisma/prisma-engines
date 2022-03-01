use barrel::types;
use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("referentialIntegrity"))]
async fn referential_integrity_parameter_is_not_added(api: &TestApi) -> TestResult {
    let result = api.re_introspect("").await?;
    assert!(!result.contains(r#"referentialIntegrity = "#));

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn multiple_changed_relation_names(api: &TestApi) -> TestResult {
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
async fn mapped_model_and_field_name(api: &TestApi) -> TestResult {
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

    let expected = expect![[
        r#"[{"code":7,"message":"These models were enriched with `@@map` information taken from the previous Prisma schema.","affected":[{"model":"Custom_User"}]},{"code":8,"message":"These fields were enriched with `@map` information taken from the previous Prisma schema.","affected":[{"model":"Post","field":"c_user_id"},{"model":"Custom_User","field":"c_id"}]}]"#
    ]];

    expected.assert_eq(&api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn multiple_changed_relation_names_due_to_mapped_models(api: &TestApi) -> TestResult {
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
