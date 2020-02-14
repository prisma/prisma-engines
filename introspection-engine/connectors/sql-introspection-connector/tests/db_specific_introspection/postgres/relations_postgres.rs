use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_one_to_one_req_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::foreign("User", "id").nullable(false).unique(true));
            });
        })
        .await;

    let dm = r#"
              model Post {
               id      Int @id @default(autoincrement())
               user_id User
            }

            model User {
               id      Int @id @default(autoincrement())
               post Post?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_two_one_to_one_relations_between_the_same_models_should_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::foreign("User", "id").unique(true).nullable(false));
            });
            migration.change_table("User", |t| {
                t.add_column("post_id", types::foreign("Post", "id").unique(true).nullable(false));
            });
        })
        .await;

    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               user_id User  @relation("Post_user_idToUser")
               user    User? @relation("PostToUser_post_id", references: [post_id])
            }

            model User {
               id      Int @id @default(autoincrement())
               post_id Post  @relation("PostToUser_post_id")
               post Post?    @relation("Post_user_idToUser")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_one_to_one_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::foreign("User", "id").unique(true).nullable(true));
            });
        })
        .await;
    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               user_id User?
            }

            model User {
               id      Int @id @default(autoincrement())
               post Post?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_one_to_one_relation_referencing_non_id_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("email TEXT UNIQUE");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_email TEXT UNIQUE REFERENCES \"User\"(\"email\")");
            });
        })
        .await;
    let dm = r#"
            model Post {
               id           Int     @id  @default(autoincrement())
               user_email   User?   @relation(references: [email])
            }

            model User {
               email        String? @unique
               id           Int     @id  @default(autoincrement())
               post         Post?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_one_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\")");
            });
        })
        .await;
    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               user_id User?
            }

            model User {
               id      Int @id @default(autoincrement())
               post Post[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_one_req_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER NOT NULL REFERENCES \"User\"(\"id\")");
            });
        })
        .await;
    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               user_id User
            }

            model User {
               id      Int @id @default(autoincrement())
               post  Post[]
            }
       "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_prisma_many_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("_PostToUser", |t| {
                t.inject_custom(
                    "A INTEGER NOT NULL REFERENCES  \"Post\"(\"id\") ON DELETE CASCADE,
                    B INTEGER NOT NULL REFERENCES  \"User\"(\"id\") ON DELETE CASCADE",
                )
            });
        })
        .await;

    api.database()
        .execute_raw(
            &format!(
                "CREATE UNIQUE INDEX test ON \"{}\".\"_PostToUser\" (\"a\", \"b\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               user  User[]
            }

            model User {
               id      Int @id @default(autoincrement())
               post  Post[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// currently disallowed by the validator since the relation tables do not have ids
//#[test_one_connector(connector = "postgres")]
//async fn introspecting_a_many_to_many_relation_should_work(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel
//        .execute(|migration| {
//            migration.create_table("User", |t| {
//                t.add_column("id", types::primary());
//            });
//            migration.create_table("Post", |t| {
//                t.add_column("id", types::primary());
//            });
//            migration.create_table("PostsToUsers", |t| {
//                t.inject_custom(
//                    "user_id INTEGER NOT NULL REFERENCES  \"User\"(\"id\") ON DELETE CASCADE,
//                    post_id INTEGER NOT NULL REFERENCES  \"Post\"(\"id\") ON DELETE CASCADE",
//                )
//            });
//        })
//        .await;
//
//    let dm = r#"
//            model Post {
//               id      Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[] @relation(references: [post_id])
//            }
//
//            model PostsToUsers {
//              post_id Post
//              user_id User
//            }
//
//            model User {
//               id      Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[]
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}
//
//#[test_one_connector(connector = "postgres")]
//async fn introspecting_a_many_to_many_relation_with_extra_fields_should_work(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel
//        .execute(|migration| {
//            migration.create_table("User", |t| {
//                t.add_column("id", types::primary());
//            });
//            migration.create_table("Post", |t| {
//                t.add_column("id", types::primary());
//            });
//            migration.create_table("PostsToUsers", |t| {
//                t.inject_custom(
//                    "date    date,
//                          user_id INTEGER NOT NULL REFERENCES  \"User\"(\"id\"),
//                    post_id INTEGER NOT NULL REFERENCES  \"Post\"(\"id\")",
//                )
//            });
//        })
//        .await;
//
//    let dm = r#"
//            model Post {
//               id      Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[] @relation(references: [post_id])
//            }
//
//            model PostsToUsers {
//              date    DateTime?
//              post_id Post
//              user_id User
//            }
//
//            model User {
//               id      Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[]
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}

#[test_one_connector(connector = "postgres")]
async fn introspecting_a_many_to_many_relation_with_an_id_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("PostsToUsers", |t| {
                t.inject_custom(
                    "id INT Primary Key,
                          user_id INTEGER NOT NULL REFERENCES  \"User\"(\"id\"),
                    post_id INTEGER NOT NULL REFERENCES  \"Post\"(\"id\")",
                )
            });
        })
        .await;

    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               postsToUsers PostsToUsers[] @relation(references: [post_id])
            }

            model PostsToUsers {
              id    Int @id
              post_id Post
              user_id User
            }

            model User {
               id      Int @id @default(autoincrement())
               postsToUsers PostsToUsers[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
#[test_each_connector(tags("postgres"))]
async fn introspecting_a_self_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "recruited_by INTEGER  REFERENCES \"User\" (\"id\"),
                     direct_report INTEGER REFERENCES \"User\" (\"id\")",
                )
            });
        })
        .await;
    let dm = r#"
            model User {
                id      Int @id @default(autoincrement())
                direct_report                  User?  @relation("UserToUser_direct_report")
                recruited_by                   User?  @relation("UserToUser_recruited_by")
                user_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                user_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// on delete cascade

// TODO: bring `onDelete` back once `prisma migrate` is a thing
//#[test_each_connector(tags("postgres"))]
async fn introspecting_cascading_delete_behaviour_should_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\") ON DELETE CASCADE");
            });
        })
        .await;

    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               user_id User?
            }

            model User {
               id    Int @id @default(autoincrement())
               post  Post[] @relation(onDelete: CASCADE)
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// enums

// native arrays

#[test_each_connector(tags("postgres"))]
async fn introspecting_default_values_on_relations_should_be_ignored(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\") Default 0");
            });
        })
        .await;

    let dm = r#"
            datasource pg {
              provider = "postgres"
              url = "postgresql://localhost:5432"
            }
            model Post {
               id      Int @id @default(autoincrement())
               user_id User?
            }

            model User {
               id      Int @id @default(autoincrement())
               post  Post[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_default_values_on_lists_should_be_ignored(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ints Integer[] DEFAULT array[]::Integer[]");
                t.inject_custom("ints2 Integer[] DEFAULT '{}'");
            });
        })
        .await;

    let dm = r#"
            datasource pg {
              provider = "postgres"
              url = "postgresql://localhost:5432"
            }

            model User {
               id      Int @id @default(autoincrement())
               ints    Int []
               ints2   Int []
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_id_fields_with_foreign_key_should_ignore_the_relation(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("test", types::text());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\") Primary Key");
            });
        })
        .await;

    let dm = r#"
            model Post {
               test    String
               user_id Int @id
            }

            model User {
               id      Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
