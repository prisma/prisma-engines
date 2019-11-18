use crate::*;
use barrel::types;
use test_harness::*;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_simple_table_with_gql_types_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("bool", types::boolean());
            t.add_column("float", types::float());
            t.add_column("date", types::date());
            t.add_column("id", types::primary());
            t.add_column("int", types::integer());
            t.add_column("string", types::text());
        });
    }).await;
    let dm = r#"
            model Blog {
                bool    Boolean
                date    DateTime
                float   Float
                id      Int @id
                int     Int 
                string  String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_table_with_compound_primary_keys_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("id", types::integer());
            t.add_column("authorId", types::varchar(10));
            t.inject_custom("PRIMARY KEY (`id`, `authorId`)");
        });
    }).await;

    let dm = r#"
            model Blog {
                authorId String
                id Int
                @@id([id, authorId])
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_table_with_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
            t.add_column("authorId", types::varchar(10));
        });
        migration.inject_custom("Create Unique Index `test` on `introspection-engine`.`Blog`( `authorId`)");
    }).await;

    let dm = r#"
            model Blog {
                authorId String @unique
                id      Int @id
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_table_with_multi_column_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("id", types::primary());
            t.add_column("firstname", types::varchar(10));
            t.add_column("lastname", types::varchar(10));
        });
        migration
            .inject_custom("Create Unique Index `test` on `introspection-engine`.`User`( `firstname`, `lastname`)");
    }).await;

    let dm = r#"
            model User {
                firstname String
                id      Int @id
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_table_with_required_and_optional_columns_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("id", types::primary());
            t.add_column("requiredname", types::text().nullable(false));
            t.add_column("optionalname", types::text().nullable(true));
        });
    }).await;

    let dm = r#"
            model User {
                id      Int @id
                optionalname String?
                requiredname String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//#[test_one_connector(connector = "mysql")]
//#[ignore]
//fn introspecting_a_table_with_datetime_default_values_should_work(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel.execute(|migration| {
//        migration.create_table("User", |t| {
//            t.add_column("id", types::primary());
//            t.add_column("name", types::text());
//            t.inject_custom("`joined` date DEFAULT CURRENT_DATE")
//        });
//    }).await;
//
//    let dm = r#"
//            model User {
//                id      Int @id
//                joined DateTime? @default(now())
//                name String
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_table_with_default_values_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("a", types::text());
            t.add_column("id", types::primary());
            t.inject_custom("`bool` Boolean NOT NULL DEFAULT false");
            t.inject_custom("`bool2` Boolean NOT NULL DEFAULT 0");
            t.inject_custom("`float` Float NOT NULL DEFAULT 5.3");
            t.inject_custom("`int` INTEGER NOT NULL DEFAULT 5");
            t.inject_custom("`string` VARCHAR(4) NOT NULL DEFAULT 'Test'");
        });
    }).await;

    let dm = r#"
            model User {
                a String
                bool Boolean @default(false)
                bool2 Boolean @default(false)
                float Float @default(5.3)
                id      Int @id
                int Int @default(5)
                string String @default("Test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_table_with_a_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("a", types::varchar(10));
            t.add_column("id", types::primary());
        });
        migration.inject_custom("Create Index `test` on `introspection-engine`.`User`(`a`)");
    }).await;

    let dm = r#"
            model User {
                a String
                id      Int @id
                @@index([a], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_table_with_a_multi_column_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("a", types::varchar(10));
            t.add_column("b", types::varchar(10));
            t.add_column("id", types::primary());
        });
        migration.inject_custom("Create Index `test` on `introspection-engine`.`User`(`a`,`b`)");
    }).await;

    let dm = r#"
            model User {
                a String
                b String
                id      Int @id
                @@index([a,b], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//relations
#[test_one_connector(connector = "mysql")]
async fn introspecting_a_one_to_one_req_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
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
    }).await;

    let dm = r#"
              model Post {
               id      Int @id
               user_id User
            }
          
            model User {
               id      Int @id
               post Post? 
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_two_one_to_one_relations_between_the_same_models_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER NOT NULL UNIQUE,\
                 FOREIGN KEY(`user_id`) REFERENCES `User`(`id`)")
            });
            migration.inject_custom("ALTER TABLE `introspection-engine`.`User` ADD Column `post_id` INTEGER NOT NULL UNIQUE ");
            migration.inject_custom("ALTER TABLE `introspection-engine`.`User` ADD CONSTRAINT `post_fk` FOREIGN KEY(`post_id`) REFERENCES `Post`(`id`)");
        }).await;

    let dm = r#"
            model Post {
               id      Int @id
               user_id User  @relation("Post_user_idToUser")
               user    User? @relation("PostToUser_post_id", references: [post_id])
            }
        
            model User {
               id      Int @id
               post_id Post  @relation("PostToUser_post_id")
               post Post?    @relation("Post_user_idToUser")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_one_to_one_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
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
    }).await;

    let dm = r#"        
            model Post {
               id      Int @id
               user_id User?
            }
            
            model User {
               id      Int @id
               post Post? 
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_one_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
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
    }).await;

    let dm = r#"  
            model Post {
               id      Int @id
               user_id User?
            }
            
            model User {
               id      Int @id
               posts Post[] 
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_one_req_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
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
    }).await;

    let dm = r#"
            model Post {
               id      Int @id
               user_id User
            }
            
            model User {
               id      Int @id
               posts Post[] 
            }
       "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_prisma_many_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
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
            )
        });
        migration.inject_custom("CREATE UNIQUE INDEX test ON `introspection-engine`.`_PostToUser` (`A`, `B`);")
    }).await;

    let dm = r#"
            model Post {
               id      Int @id
               users User[] 
            }
            
            model User {
               id      Int @id
               posts Post[] 
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_many_to_many_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("id", types::primary());
        });
        migration.create_table("Post", |t| {
            t.add_column("id", types::primary());
        });
        migration.create_table("PostsToUsers", |t| {
            t.inject_custom(
                "user_id INTEGER NOT NULL,
                     post_id INTEGER NOT NULL,
                     FOREIGN KEY (`user_id`) REFERENCES  `User`(`id`) ON DELETE CASCADE,
                     FOREIGN KEY (`post_id`) REFERENCES  `Post`(`id`) ON DELETE CASCADE",
            )
        });
    }).await;

    let dm = r#"
            model Post {
               id      Int @id
               postsToUserses PostsToUsers[] @relation(references: [post_id], onDelete: CASCADE)
            }

            model PostsToUsers {
              post_id Post 
              user_id User
            }
            
            model User {
               id      Int @id
               postsToUserses PostsToUsers[] @relation(onDelete: CASCADE)
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn introspecting_a_many_to_many_relation_with_extra_fields_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("id", types::primary());
        });
        migration.create_table("Post", |t| {
            t.add_column("id", types::primary());
        });
        migration.create_table("PostsToUsers", |t| {
            t.inject_custom(
                "date    date,
                     user_id INTEGER NOT NULL,
                     post_id INTEGER NOT NULL,
                     FOREIGN KEY (`user_id`) REFERENCES  `User`(`id`),
                     FOREIGN KEY (`post_id`) REFERENCES  `Post`(`id`)",
            )
        });
    }).await;

    let dm = r#"
            model Post {
               id      Int @id
               postsToUserses PostsToUsers[] @relation(references: [post_id])
            }
            
            model PostsToUsers {
              date    DateTime?
              post_id Post 
              user_id User
            }
            
            model User {
               id      Int @id
               postsToUserses PostsToUsers[] 
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
#[test_one_connector(connector = "mysql")]
async fn introspecting_a_self_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("id", types::primary());
            t.inject_custom(
                "recruited_by INTEGER, 
                     direct_report INTEGER,
                     FOREIGN KEY (`recruited_by`) REFERENCES `User` (`id`),
                     FOREIGN KEY (`direct_report`) REFERENCES `User` (`id`)",
            )
        });
    }).await;

    let dm = r#"
            model User {
                direct_report                  User?  @relation("UserToUser_direct_report")
                id      Int @id
                recruited_by                   User?  @relation("UserToUser_recruited_by")
                users_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                users_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// on delete cascade

#[test_one_connector(connector = "mysql")]
async fn introspecting_cascading_delete_behaviour_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("User", |t| {
            t.add_column("id", types::primary());
        });
        migration.create_table("Post", |t| {
            t.add_column("id", types::primary());
            t.inject_custom(
                "user_id INTEGER,\
                 FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE CASCADE",
            );
        });
    }).await;

    let dm = r#"  
            model Post {
               id      Int @id
               user_id User?
            }
            
            model User {
               id      Int @id
               posts Post[] @relation(onDelete: CASCADE)
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// enums

// native arrays
