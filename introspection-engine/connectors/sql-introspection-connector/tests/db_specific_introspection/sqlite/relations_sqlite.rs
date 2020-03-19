use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_one_to_one_req_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "user_id INTEGER NOT NULL UNIQUE,
                        FOREIGN KEY(user_id) REFERENCES User(id)",
                )
            });
        })
        .await;

    let dm = r#"
            model User {
               id Int @id @default(autoincrement())
               post Post?
            }

            model Post {
               id Int @id @default(autoincrement())
               user_id User
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_two_one_to_one_relations_between_the_same_models_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "post_id INTEGER NOT NULL UNIQUE,
                        FOREIGN KEY(post_id) REFERENCES Post(id)",
                )
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "user_id INTEGER NOT NULL UNIQUE,
                        FOREIGN KEY(user_id) REFERENCES User(id)",
                )
            });
        })
        .await;

    let dm = r#"
            model User {
               id Int @id @default(autoincrement())
               post_id Post  @relation("PostToUser_post_id", references: [id])
               post Post?    @relation("Post_user_idToUser")
            }

            model Post {
               id Int @id @default(autoincrement())
               user_id User  @relation("Post_user_idToUser", references: [id])
               user    User? @relation("PostToUser_post_id")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_one_to_one_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "user_id INTEGER UNIQUE,
                        FOREIGN KEY(user_id) REFERENCES User(id)",
                )
            });
        })
        .await;

    let dm = r#"
            model User {
               id Int @id @default(autoincrement())
               post Post?
            }

            model Post {
               id Int @id @default(autoincrement())
               user_id User?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_one_to_one_relation_referencing_non_id_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("email varchar(10) UNIQUE");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "user_email varchar(10) UNIQUE,
                FOREIGN KEY (user_email) REFERENCES User(email)",
                );
            });
        })
        .await;
    let dm = r#"
            model User {
               email        String? @unique
               id           Int     @id  @default(autoincrement())
               post         Post?
            }

            model Post {
               id           Int     @id   @default(autoincrement())
               user_email   User?   @relation(references: [email])
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_one_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "user_id INTEGER,
                        FOREIGN KEY(user_id) REFERENCES User(id)",
                )
            });
        })
        .await;

    let dm = r#"
            model User {
               id Int @id @default(autoincrement())
               post Post[]
            }

            model Post {
               id Int @id @default(autoincrement())
               user_id User?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_one_req_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "user_id INTEGER NOT NULL,
                        FOREIGN KEY(user_id) REFERENCES User(id)",
                )
            });
        })
        .await;

    let dm = r#"
            model User {
               id Int @id @default(autoincrement())
               post Post[]
            }

            model Post {
               id Int @id @default(autoincrement())
               user_id User
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
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
                    "
                        A TEXT NOT NULL,
                        B TEXT NOT NULL,
                        FOREIGN KEY (A) REFERENCES  Post(id) ON DELETE CASCADE,
                        FOREIGN KEY (B) REFERENCES  User(id) ON DELETE CASCADE
                    ",
                )
            });
        })
        .await;

    api.database()
        .execute_raw(
            &format!(
                "CREATE UNIQUE INDEX \"{}\".test ON \"_PostToUser\" (\"A\", \"B\");",
                api.schema_name(),
            ),
            &[],
        )
        .await
        .unwrap();

    let dm = r#"
            model User {
               id Int @id @default(autoincrement())
               post Post[]
            }

            model Post {
               id Int @id @default(autoincrement())
               user User[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// currently disallowed by the validator since the relation tables do not have ids
//#[test_each_connector(tags("sqlite"))]
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
//                    "user_id TEXT NOT NULL,
//                          post_id TEXT NOT NULL,
//                          FOREIGN KEY (user_id) REFERENCES  User(id) ON DELETE CASCADE,
//                          FOREIGN KEY (post_id) REFERENCES  Post(id) ON DELETE CASCADE",
//                )
//            });
//        })
//        .await;
//
//    let dm = r#"
//            model User {
//               id Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[]
//            }
//
//            model Post {
//               id Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[] @relation(references: [post_id])
//            }
//
//            model PostsToUsers {
//              post_id Post
//              user_id User
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}
//
//#[test_each_connector(tags("sqlite"))]
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
//                          user_id TEXT NOT NULL,
//                          post_id TEXT NOT NULL,
//                          FOREIGN KEY (user_id) REFERENCES  User(id),
//                          FOREIGN KEY (post_id) REFERENCES  Post(id)",
//                )
//            });
//        })
//        .await;
//
//    let dm = r#"
//            model User {
//               id Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[]
//            }
//
//            model Post {
//               id Int @id @default(autoincrement())
//               postsToUserses PostsToUsers[] @relation(references: [post_id])
//            }
//
//            model PostsToUsers {
//              date    DateTime?
//              post_id Post
//              user_id User
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}

#[test_each_connector(tags("sqlite"))]
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
                    "id Integer Primary Key Not Null,
                          user_id TEXT NOT NULL,
                          post_id TEXT NOT NULL,
                          FOREIGN KEY (user_id) REFERENCES  User(id),
                          FOREIGN KEY (post_id) REFERENCES  Post(id)",
                )
            });
        })
        .await;

    let dm = r#"
            model User {
               id Int @id @default(autoincrement())
               postsToUsers PostsToUsers[]
            }

            model Post {
               id Int @id @default(autoincrement())
               postsToUsers PostsToUsers[] 
            }

            model PostsToUsers {
              id      Int @id @default(autoincrement())
              post_id Post
              user_id User
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_self_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "recruited_by INTEGER,
                          direct_report INTEGER,
                          FOREIGN KEY (recruited_by) REFERENCES User (id),
                          FOREIGN KEY (direct_report) REFERENCES User (id)",
                )
            });
        })
        .await;

    let dm = r#"
            model User {
                id                             Int    @id  @default(autoincrement())
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
//#[test_each_connector(tags("sqlite"))]
async fn introspecting_cascading_delete_behaviour_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "user_id INTEGER,\
                     FOREIGN KEY (user_id) REFERENCES User(id) ON DELETE CASCADE",
                );
            });
        })
        .await;

    let dm = r#"
            model User {
               id      Int @id @default(autoincrement())
               post Post[] @relation(onDelete: CASCADE)
            }

            model Post {
               id      Int @id @default(autoincrement())
               user_id User?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_id_fields_with_foreign_key_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("test", types::text());
                t.inject_custom("user_id INTEGER Not Null Primary Key");
                t.inject_custom("FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)");
            });
        })
        .await;

    let dm = r#"
            model User {
               id      Int       @id @default(autoincrement())
               post    Post[]
            }

            model Post {
               test    String
               user_id User      @id @relation(references: [id])
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
