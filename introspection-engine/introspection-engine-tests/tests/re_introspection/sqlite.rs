use barrel::types;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Sqlite))]
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
          custom_User  Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          custom_User2 Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], onDelete: NoAction, onUpdate: NoAction)
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
