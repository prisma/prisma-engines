use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Sqlite))]
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
                t.add_column(
                    "createdAt",
                    types::custom("INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))"),
                );

                t.add_foreign_key(&["user_id"], "User", &["id"]);
                t.add_foreign_key(&["user_id2"], "User", &["id"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = r#"
        model Post {
            id               Int @id @default(autoincrement())
            user_id          Int  @unique
            user_id2         Int  @unique
            createdAt        Int  @default(dbgenerated("(CAST(unixepoch('subsec') * 1000 AS INTEGER))"))
            custom_User      Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
            custom_User2     Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Custom_User {
            id               Int @id @default(autoincrement())
            custom_Post      Post? @relation("CustomRelationName")
            custom_Post2     Post? @relation("AnotherCustomRelationName")

            @@map("User")
        }
    "#;

    let expected = expect![[r#"
        model Post {
          id           Int         @id @default(autoincrement())
          user_id      Int         @unique(map: "sqlite_autoindex_Post_1")
          user_id2     Int         @unique(map: "sqlite_autoindex_Post_2")
          createdAt    Int         @default(dbgenerated("(CAST(unixepoch('subsec') * 1000 AS INTEGER))"))
          custom_User2 Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], onDelete: NoAction, onUpdate: NoAction)
          custom_User  Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Custom_User {
          id           Int   @id @default(autoincrement())
          custom_Post2 Post? @relation("AnotherCustomRelationName")
          custom_Post  Post? @relation("CustomRelationName")

          @@map("User")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn do_not_try_to_keep_custom_many_to_many_self_relation_field_names(api: &mut TestApi) -> TestResult {
    // We do not have enough information to correctly assign which field should point to column A in the
    // join table and which one to B
    // Upon table creation this is dependant on lexicographic order of the names of the fields, but we
    // cannot be sure that users keep the order the same when renaming. worst case would be we accidentally
    // switch the directions when reintrospecting.
    // The generated names are also not helpful though, but at least they don't give a false sense of correctness -.-
    let sql = r#"
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "_FollowRelation" (
            "A" INTEGER NOT NULL REFERENCES "User"("id"),
            "B" INTEGER NOT NULL REFERENCES "User"("id")
        );

        CREATE UNIQUE INDEX "_FollowRelation_AB_unique" ON "_FollowRelation"("A", "B");
        CREATE INDEX "_FollowRelation_B_index" ON "_FollowRelation"("B");
    "#;

    api.raw_cmd(sql).await;

    let input_dm = indoc! {r#"
        model User {
            id          Int       @id @default(autoincrement())
            followers   User[]    @relation("FollowRelation")
            following   User[]    @relation("FollowRelation")
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id     Int    @id @default(autoincrement())
          User_A User[] @relation("FollowRelation")
          User_B User[] @relation("FollowRelation")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn multiple_changed_relation_names(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "Employee" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "Schedule" (
            id INTEGER PRIMARY KEY,
            "morningEmployeeId" INTEGER NOT NULL REFERENCES "Employee"("id"),
            "eveningEmployeeId" INTEGER NOT NULL REFERENCES "Employee"("id")
        );

        CREATE TABLE "Unrelated" (
            id INTEGER PRIMARY KEY
        );
    "#;
    api.raw_cmd(sql).await;

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
            Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }
    "#};

    let expectation = expect![[r#"
        model Employee {
          id                                            Int        @id @default(autoincrement())
          A                                             Schedule[] @relation("EmployeeToSchedule_eveningEmployeeId")
          Schedule_EmployeeToSchedule_morningEmployeeId Schedule[] @relation("EmployeeToSchedule_morningEmployeeId")
        }

        model Schedule {
          id                                            Int      @id @default(autoincrement())
          morningEmployeeId                             Int
          eveningEmployeeId                             Int
          Employee_EmployeeToSchedule_eveningEmployeeId Employee @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
          Employee_EmployeeToSchedule_morningEmployeeId Employee @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}
