use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
async fn a_one_to_one_relation_referencing_non_id(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("email", types::varchar(10).nullable(true));

                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
                t.add_constraint("User_email_key", types::unique_constraint(vec!["email"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_email", types::varchar(10).nullable(true));
                t.add_constraint(
                    "Post_user_email_fkey",
                    types::foreign_constraint(&["user_email"], "User", &["email"], None, None),
                );

                t.add_constraint("Post_pkey", types::primary_constraint(vec!["id"]));
                t.add_constraint("Post_user_email_key", types::unique_constraint(vec!["user_email"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id         BigInt  @id @default(autoincrement())
          user_email String? @unique @db.String(10)
          User       User?   @relation(fields: [user_email], references: [email], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id    BigInt  @id @default(autoincrement())
          email String? @unique @db.String(10)
          Post  Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn default_values_on_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\") Default 0");
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      BigInt  @id @default(autoincrement())
          user_id BigInt? @default(0)
          User    User?   @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   BigInt @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn a_self_relation(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("recruited_by", types::integer().nullable(true));
                t.add_column("direct_report", types::integer().nullable(true));

                t.add_constraint(
                    "recruited_by_fkey",
                    types::foreign_constraint(&["recruited_by"], "User", &["id"], None, None),
                );
                t.add_constraint(
                    "direct_report_fkey",
                    types::foreign_constraint(&["direct_report"], "User", &["id"], None, None),
                );
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model User {
          id                                  BigInt  @id @default(autoincrement())
          recruited_by                        BigInt?
          direct_report                       BigInt?
          User_User_direct_reportToUser       User?   @relation("User_direct_reportToUser", fields: [direct_report], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "direct_report_fkey")
          other_User_User_direct_reportToUser User[]  @relation("User_direct_reportToUser")
          User_User_recruited_byToUser        User?   @relation("User_recruited_byToUser", fields: [recruited_by], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "recruited_by_fkey")
          other_User_User_recruited_byToUser  User[]  @relation("User_recruited_byToUser")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn a_many_to_many_relation_with_an_id(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("PostsToUsers", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("post_id", types::integer().nullable(false));

                t.add_foreign_key(&["user_id"], "User", &["id"]);
                t.add_foreign_key(&["post_id"], "Post", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id           BigInt         @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }

        model PostsToUsers {
          id      BigInt @id @default(autoincrement())
          user_id BigInt
          post_id BigInt
          Post    Post   @relation(fields: [post_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          User    User   @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id           BigInt         @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
