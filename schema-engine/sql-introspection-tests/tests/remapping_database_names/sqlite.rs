use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Sqlite))]
async fn remapping_models_in_compound_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User with Space", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_constraint(
                    "sqlite_autoindex_User with Space_1",
                    types::unique_constraint(vec!["id", "age"]),
                );
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_foreign_key(&["user_id", "user_age"], "User with Space", &["id", "age"]);

                t.add_constraint(
                    "sqlite_autoindex_Post_1",
                    types::unique_constraint(vec!["user_id", "user_age"]).unique(true),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id              Int             @id @default(autoincrement())
          user_id         Int
          user_age        Int
          User_with_Space User_with_Space @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)

          @@unique([user_id, user_age], map: "sqlite_autoindex_Post_1")
        }

        model User_with_Space {
          id   Int   @id @default(autoincrement())
          age  Int
          Post Post?

          @@unique([id, age], map: "sqlite_autoindex_User with Space_1")
          @@map("User with Space")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn remapping_models_in_relations(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());

                if sql_family.is_mysql() {
                    t.inject_custom(
                        "CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES `User with Space`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                    );
                } else {
                    t.inject_custom(
                        r#"CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES "User with Space"(id) ON DELETE RESTRICT ON UPDATE CASCADE"#,
                    );
                }

                t.add_constraint(
                    "post_user_unique",
                    types::unique_constraint(vec!["user_id"]).unique(true),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id              Int             @id @default(autoincrement())
          user_id         Int             @unique(map: "sqlite_autoindex_Post_1")
          User_with_Space User_with_Space @relation(fields: [user_id], references: [id])
        }

        model User_with_Space {
          id   Int   @id @default(autoincrement())
          Post Post?

          @@map("User with Space")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn remapping_models_in_relations_should_not_map_virtual_fields(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post With Space", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());

                if sql_family.is_mysql() {
                    t.inject_custom(
                        "CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                    );
                } else {
                    t.inject_custom(
                        r#"CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES "User"(id) ON DELETE RESTRICT ON UPDATE CASCADE"#,
                    );
                }


                t.add_constraint("post_user_unique", types::unique_constraint(vec!["user_id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post_With_Space {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique(map: "sqlite_autoindex_Post With Space_1")
          User    User @relation(fields: [user_id], references: [id])

          @@map("Post With Space")
        }

        model User {
          id              Int              @id @default(autoincrement())
          Post_With_Space Post_With_Space?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
