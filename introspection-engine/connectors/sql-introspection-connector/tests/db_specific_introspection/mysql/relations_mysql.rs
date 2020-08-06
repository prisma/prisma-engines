use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_one_to_one_req_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom(
                        "user_id INTEGER NOT NULL UNIQUE,
                FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)",
                    )
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
               id       Int @id @default(autoincrement())
               user_id  Int  @unique
               User     User @relation(fields: [user_id], references: [id])
            }

            model User {
               id      Int @id @default(autoincrement())
               Post Post?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_two_one_to_one_relations_between_the_same_models_should_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("post_id", types::integer().nullable(false).unique(true));
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom(
                        "user_id INTEGER NOT NULL UNIQUE,\
                         FOREIGN KEY(`user_id`) REFERENCES `User`(`id`)",
                    )
                });
            },
            api.db_name(),
        )
        .await;

    barrel
        .execute_with_schema(
            |migration| {
                migration.change_table("User", |t| {
                    t.inject_custom("ADD CONSTRAINT `post_fk` FOREIGN KEY(`post_id`) REFERENCES `Post`(`id`)");
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
        model Post {
            id                      Int   @default(autoincrement()) @id
            user_id                 Int   @unique
            User_Post_user_idToUser User  @relation("Post_user_idToUser", fields: [user_id], references: [id])
            User_PostToUser_post_id User? @relation("PostToUser_post_id")
        }
                
        model User {
            id                      Int   @default(autoincrement()) @id
            post_id                 Int   @unique
            Post_PostToUser_post_id Post  @relation("PostToUser_post_id", fields: [post_id], references: [id])
            Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
        }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_one_to_one_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom(
                        "user_id INTEGER UNIQUE,\
                         FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)",
                    );
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id      Int   @default(autoincrement()) @id
                user_id Int?  @unique
                User    User? @relation(fields: [user_id], references: [id])
            }
                  
            model User {
                id   Int   @default(autoincrement()) @id
                Post Post?
            }       
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
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
                FOREIGN KEY (`user_email`) REFERENCES `User`(`email`)",
                );
            });
        })
        .await;
    let dm = r#"
           model Post {
                id         Int     @default(autoincrement()) @id
                user_email String? @unique
                User       User?   @relation(fields: [user_email], references: [email])
            }
                  
            model User {
                id    Int     @default(autoincrement()) @id
                email String? @unique
                Post  Post?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_one_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom(
                        "user_id INTEGER,\
                         FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)",
                    );
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id      Int   @default(autoincrement()) @id
                user_id Int?
                User    User? @relation(fields: [user_id], references: [id])
                
                @@index([user_id], name: "user_id")
            }
            
            model User {
                id   Int    @default(autoincrement()) @id
                Post Post[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_one_req_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom(
                        "user_id INTEGER NOT NULL,\
                         FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)",
                    );
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id      Int  @default(autoincrement()) @id
                user_id Int
                User    User @relation(fields: [user_id], references: [id])
                
                @@index([user_id], name: "user_id")
            }
            
            model User {
                id   Int    @default(autoincrement()) @id
                Post Post[]
            }
       "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_prisma_many_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("_PostToUser", |t| {
                    t.inject_custom(
                        "A INTEGER NOT NULL,
                     B INTEGER NOT NULL,
                     FOREIGN KEY (`A`) REFERENCES  `Post`(`id`) ON DELETE CASCADE,
                     FOREIGN KEY (`B`) REFERENCES  `User`(`id`) ON DELETE CASCADE",
                    );
                    t.add_index("test", types::index(vec!["A", "B"]).unique(true));
                    t.add_index("test2", types::index(vec!["B"]).unique(false));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"model Post {
  id   Int    @default(autoincrement()) @id
  User User[]
}

model User {
  id   Int    @default(autoincrement()) @id
  Post Post[]
}
"#;
    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_many_to_many_relation_with_an_id_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("PostsToUsers", |t| {
                    t.inject_custom(
                        "id INTEGER PRIMARY KEY,
                     user_id INTEGER NOT NULL,
                     post_id INTEGER NOT NULL,
                     FOREIGN KEY (`user_id`) REFERENCES  `User`(`id`),
                     FOREIGN KEY (`post_id`) REFERENCES  `Post`(`id`)",
                    )
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id           Int            @default(autoincrement()) @id
                PostsToUsers PostsToUsers[]
            }
            
            model PostsToUsers {
                id      Int  @id
                user_id Int
                post_id Int
                Post    Post @relation(fields: [post_id], references: [id])
                User    User @relation(fields: [user_id], references: [id])
                
                @@index([post_id], name: "post_id")
                @@index([user_id], name: "user_id")
            }
            
            model User {
                id           Int            @default(autoincrement()) @id
                PostsToUsers PostsToUsers[]
            }         
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_self_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom(
                        "recruited_by INTEGER,
                     direct_report INTEGER,
                     FOREIGN KEY (`recruited_by`) REFERENCES `User` (`id`),
                     FOREIGN KEY (`direct_report`) REFERENCES `User` (`id`)",
                    )
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
              model User {
                id                                  Int    @default(autoincrement()) @id
                recruited_by                        Int?
                direct_report                       Int?
                User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id])
                User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id])
                other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
                
                @@index([direct_report], name: "direct_report")
                @@index([recruited_by], name: "recruited_by")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_id_fields_with_foreign_key_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("test", types::text());
                t.inject_custom("user_id INTEGER Primary Key");
                t.inject_custom("FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)");
            });
        })
        .await;

    let dm = r#"
        model Post {
            test    String
            user_id Int    @id
            User    User   @relation(fields: [user_id], references: [id])
        }
              
        model User {
            id   Int    @default(autoincrement()) @id
            Post Post[]
        }
"#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// on delete cascade

// TODO: bring `onDelete` back once `prisma migrate` is a thing
//#[test_each_connector(tags("mysql"))]
// async fn introspecting_cascading_delete_behaviour_should_work(api: &TestApi) {
//     let barrel = api.barrel();
//     let _setup_schema = barrel
//         .execute_with_schema(
//             |migration| {
//                 migration.create_table("User", |t| {
//                     t.add_column("id", types::primary());
//                 });
//                 migration.create_table("Post", |t| {
//                     t.add_column("id", types::primary());
//                     t.inject_custom(
//                         "user_id INTEGER, FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE CASCADE",
//                     );
//                 });
//             },
//             api.db_name(),
//         )
//         .await;
//
//     let dm = r#"
//             model Post {
//                id      Int @id @default(autoincrement())
//                user_id User?
//
//                @@index([user_id], name: "user_id")
//             }
//
//             model User {
//                id      Int @id @default(autoincrement())
//                Post Post[] @relation(onDelete: CASCADE)
//             }
//         "#;
//     let result = api.introspect().await;
//     custom_assert(&result, dm);
// }

// currently disallowed by the validator since the relation tables do not have ids
//#[test_each_connector(tags("mysql"))]
//async fn introspecting_a_many_to_many_relation_should_work(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel
//        .execute_with_schema(
//            |migration| {
//                migration.create_table("User", |t| {
//                    t.add_column("id", types::primary());
//                });
//                migration.create_table("Post", |t| {
//                    t.add_column("id", types::primary());
//                });
//                migration.create_table("PostsToUsers", |t| {
//                    t.inject_custom(
//                        "user_id INTEGER NOT NULL,
//                     post_id INTEGER NOT NULL,
//                     FOREIGN KEY (`user_id`) REFERENCES  `User`(`id`) ON DELETE CASCADE,
//                     FOREIGN KEY (`post_id`) REFERENCES  `Post`(`id`) ON DELETE CASCADE",
//                    )
//                });
//            },
//            api.db_name(),
//        )
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
//
//              @@index([post_id], name: "post_id")
//              @@index([user_id], name: "user_id")
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
//#[test_each_connector(tags("mysql"))]
//async fn introspecting_a_many_to_many_relation_with_extra_fields_should_work(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel
//        .execute_with_schema(
//            |migration| {
//                migration.create_table("User", |t| {
//                    t.add_column("id", types::primary());
//                });
//                migration.create_table("Post", |t| {
//                    t.add_column("id", types::primary());
//                });
//                migration.create_table("PostsToUsers", |t| {
//                    t.inject_custom(
//                        "date    date,
//                     user_id INTEGER NOT NULL,
//                     post_id INTEGER NOT NULL,
//                     FOREIGN KEY (`user_id`) REFERENCES  `User`(`id`),
//                     FOREIGN KEY (`post_id`) REFERENCES  `Post`(`id`)",
//                    )
//                });
//            },
//            api.db_name(),
//        )
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
//
//              @@index([post_id], name: "post_id")
//              @@index([user_id], name: "user_id")
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
