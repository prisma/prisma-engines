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
                id         Int     @default(autoincrement()) @id
                user_email String? @unique
                User       User?   @relation(fields: [user_email], references: [email])
            }
                  
            model User {
                email String? @unique
                id    Int     @default(autoincrement()) @id
                Post  Post?
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
                id      Int   @default(autoincrement()) @id
                user_id Int?
                User    User? @relation(fields: [user_id], references: [id])
            }
            
            model User {
                id   Int    @default(autoincrement()) @id
                Post Post[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_duplicate_fks_should_ignore_one_of_them(api: &TestApi) {
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

    api.database()
        .execute_raw(
            &format!(
                "Alter table \"{}\".\"Post\" ADD CONSTRAINT fk_duplicate FOREIGN KEY (\"user_id\") REFERENCES \"User\" (\"id\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    let dm = r#"
             model Post {
                 id      Int   @default(autoincrement()) @id
                 user_id Int?
                 User    User? @relation("Post_user_idToUser", fields: [user_id], references: [id])
             }
             
             model User {
                 id   Int    @default(autoincrement()) @id
                 Post Post[] @relation("Post_user_idToUser")
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
                id      Int  @default(autoincrement()) @id
                user_id Int
                User    User @relation(fields: [user_id], references: [id])
            }
            
            model User {
                id   Int    @default(autoincrement()) @id
                Post Post[]
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

    api.database()
        .execute_raw(
            &format!(
                "CREATE INDEX test2 ON \"{}\".\"_PostToUser\" (\"b\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    let dm = r#"
            model Post {
               id      Int @id @default(autoincrement())
               User  User[]
            }

            model User {
               id      Int @id @default(autoincrement())
               Post  Post[]
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
                id           Int            @default(autoincrement()) @id
                PostsToUsers PostsToUsers[]
            }
            
            model PostsToUsers {
                id      Int  @id
                post_id Int
                user_id Int
                Post    Post @relation(fields: [post_id], references: [id])
                User    User @relation(fields: [user_id], references: [id])
            }
            
            model User {
                id           Int            @default(autoincrement()) @id
                PostsToUsers PostsToUsers[]
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
                direct_report                       Int?
                id                                  Int    @default(autoincrement()) @id
                recruited_by                        Int?
                User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id])
                User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id])
                other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// on delete cascade

// TODO: bring `onDelete` back once `prisma migrate` is a thing
//#[test_each_connector(tags("postgres"))]
// async fn introspecting_cascading_delete_behaviour_should_work(api: &TestApi) {
//     let barrel = api.barrel();
//     barrel
//         .execute(|migration| {
//             migration.create_table("User", |t| {
//                 t.add_column("id", types::primary());
//             });
//             migration.create_table("Post", |t| {
//                 t.add_column("id", types::primary());
//                 t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\") ON DELETE CASCADE");
//             });
//         })
//         .await;
//
//     let dm = r#"
//             model Post {
//                id      Int @id @default(autoincrement())
//                user_id User?
//             }
//
//             model User {
//                id    Int @id @default(autoincrement())
//                Post  Post[] @relation(onDelete: CASCADE)
//             }
//         "#;
//     let result = dbg!(api.introspect().await);
//     custom_assert(&result, dm);
// }

#[test_each_connector(tags("postgres"))]
async fn introspecting_default_values_on_relations_should_work(api: &TestApi) {
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
            model Post {
                id      Int   @default(autoincrement()) @id
                user_id Int?  @default(0)
                User    User? @relation(fields: [user_id], references: [id])
            }
            
            model User {
                id   Int    @default(autoincrement()) @id
                Post Post[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_id_fields_with_foreign_key_should_work(api: &TestApi) {
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

#[test_each_connector(tags("postgres"))]
async fn introspecting_prisma_10_relations_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id CHAR(25) NOT NULL PRIMARY KEY");
            });
            migration.create_table("Royalty", |t| {
                t.inject_custom("id CHAR(25) NOT NULL PRIMARY KEY");
            });
            migration.create_table("_BookRoyalty", |t| {
                t.inject_custom("id CHAR(25) NOT NULL PRIMARY KEY");
                t.inject_custom("A CHAR(25) NOT NULL REFERENCES \"Book\"(\"id\")");
                t.inject_custom("B CHAR(25) NOT NULL REFERENCES \"Royalty\"(\"id\")");
            });
        })
        .await;

    api.database()
        .execute_raw(
            &format!(
                "CREATE UNIQUE INDEX  double on \"{}\".\"_BookRoyalty\" (\"a\", \"b\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    api.database()
        .execute_raw(
            &format!(
                "CREATE INDEX single on \"{}\".\"_BookRoyalty\" (\"b\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    let dm = r#"
            model Book {
              id        String      @id
              Royalty   Royalty[]   @relation("BookRoyalty")
            }
                
            model Royalty {
              id        String      @id
              Book      Book[]      @relation("BookRoyalty")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_relations_should_avoid_name_clashes(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("y", |t| {
                t.inject_custom("id CHAR(25) NOT NULL PRIMARY KEY");
                t.inject_custom("x  CHAR(25) NOT NULL");
            });
            migration.create_table("x", |t| {
                t.inject_custom("id CHAR(25) NOT NULL PRIMARY KEY");
                t.inject_custom("y CHAR(25) NOT NULL REFERENCES \"y\"(\"id\")");
            });
        })
        .await;

    let dm = r#"
            model x {
                id String @id
                y  String
                y_xToy  y      @relation(fields: [y], references: [id])
            }
                  
            model y {
                id String @id
                x  String
                x_xToy  x[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//
// CREATE TABLE IF NOT EXISTS `x` (
// `y` int(11) DEFAULT NULL,
// `id` int(11) DEFAULT NULL,
// UNIQUE KEY `unique_id` (`id`) USING BTREE,
// UNIQUE KEY `unique_y_id` (`y`,`id`) USING BTREE,
// KEY `FK__y` (`y`),
// CONSTRAINT `FK__y` FOREIGN KEY (`y`) REFERENCES `y` (`id`)
// ) ENGINE=InnoDB DEFAULT;
//
// CREATE TABLE IF NOT EXISTS `y` (
// `id` int(11) DEFAULT NULL,
// `x` int(11) DEFAULT NULL,
// `fk_x_1` int(11) DEFAULT NULL,
// `fk_x_2` int(11) DEFAULT NULL,
// UNIQUE KEY `unique_id` (`id`) USING BTREE,
// KEY `FK_y_x` (`fk_x_1`,`fk_x_2`) USING BTREE,
// CONSTRAINT `FK_y_x` FOREIGN KEY (`fk_x_1`, `fk_x_2`) REFERENCES `x` (`y`, `id`)
// ) ENGINE=InnoDB DEFAULT;

#[test_each_connector(tags("postgres"))]
async fn introspecting_relations_should_avoid_name_clashes_2(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("x", |t| {
                t.inject_custom("id CHAR(25) NOT NULL PRIMARY KEY");
                t.inject_custom("y CHAR(25) NOT NULL");
            });

            migration.create_table("y", |t| {
                t.inject_custom("id CHAR(25) NOT NULL PRIMARY KEY");
                t.inject_custom("x  CHAR(25) NOT NULL");
                t.inject_custom("fk_x_1  CHAR(25) NOT NULL");
                t.inject_custom("fk_x_2  CHAR(25) NOT NULL");
            });
        })
        .await;

    api.database()
        .execute_raw(
            &format!(
                "CREATE UNIQUE INDEX unique_y_id on \"{}\".\"x\" (\"id\", \"y\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    api.database()
        .execute_raw(
            &format!(
                "Alter table \"{}\".\"x\" ADD CONSTRAINT fk_y FOREIGN KEY (\"y\") REFERENCES \"y\" (\"id\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    api.database()
        .execute_raw(
            &format!(
                "Alter table \"{}\".\"y\" ADD CONSTRAINT fk_y_x FOREIGN KEY (\"fk_x_1\", \"fk_x_2\") REFERENCES \"x\" (\"y\",\"id\");",
                api.schema_name()
            ),
            &[],
        )
        .await
        .unwrap();

    let dm = r#"
            model x {
                id                   String @id
                y                    String
                y_x_yToy             y      @relation("x_yToy", fields: [y], references: [id])
                y_xToy_fk_x_1_fk_x_2 y[]    @relation("xToy_fk_x_1_fk_x_2")
                    
                @@unique([id, y], name: "unique_y_id")
            }
                      
            model y {
               fk_x_1               String
               fk_x_2               String
               id                   String @id
               x                    String
               x_xToy_fk_x_1_fk_x_2 x      @relation("xToy_fk_x_1_fk_x_2", fields: [fk_x_1, fk_x_2], references: [y, id])
               x_x_yToy             x[]    @relation("x_yToy")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_relation_based_on_an_unsupported_type_should_drop_it(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("network_mac  macaddr Not null Unique");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_network_mac macaddr REFERENCES \"User\"(\"network_mac\")");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(
        &warnings,
        "[{\"code\":3,\"message\":\"These fields were commented out because we currently do not support their types.\",\"affected\":[{\"model\":\"Post\",\"field\":\"user_network_mac\",\"tpe\":\"macaddr\"},{\"model\":\"User\",\"field\":\"network_mac\",\"tpe\":\"macaddr\"}]}]"
    );

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model Post {\n  id                  Int      @default(autoincrement()) @id\n  // This type is currently not supported.\n  // user_network_mac macaddr?\n}\n\nmodel User {\n  id             Int     @default(autoincrement()) @id\n  // This type is currently not supported.\n  // network_mac macaddr @unique\n}");
}
