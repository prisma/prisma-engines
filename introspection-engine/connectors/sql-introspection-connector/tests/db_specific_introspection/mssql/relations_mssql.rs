use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_one_to_one_req_relation_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();

    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", move |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("user_id INT NOT NULL UNIQUE");
                    t.inject_custom(&format!("FOREIGN KEY ([user_id]) REFERENCES [{}].[User]([id])", schema));
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

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_two_one_to_one_relations_between_the_same_models_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();

    barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("post_id", types::integer().nullable(false).unique(true));
                });
                migration.create_table("Post", move |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("user_id INTEGER NOT NULL UNIQUE");
                    t.inject_custom(&format!("FOREIGN KEY([user_id]) REFERENCES [{}].[User]([id])", schema));
                });
            },
            api.db_name(),
        )
        .await;

    let schema = api.schema_name().to_string();

    barrel
        .execute_with_schema(
            move |migration| {
                migration.change_table("User", move |t| {
                    t.inject_custom(&format!(
                        "ADD CONSTRAINT [post_fk] FOREIGN KEY([post_id]) REFERENCES [{}].[Post]([id])",
                        &schema
                    ));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
        model Post {
            id                      Int   @id @default(autoincrement())
            user_id                 Int   @unique
            User_Post_user_idToUser User  @relation("Post_user_idToUser", fields: [user_id], references: [id])
            User_PostToUser_post_id User? @relation("PostToUser_post_id")
        }
                
        model User {
            id                      Int   @id @default(autoincrement())
            post_id                 Int   @unique
            Post_PostToUser_post_id Post  @relation("PostToUser_post_id", fields: [post_id], references: [id])
            Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
        }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_one_to_one_relation_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", move |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("[user_id] INT UNIQUE");
                    t.inject_custom(&format!("FOREIGN KEY ([user_id]) REFERENCES [{}].[User]([id])", schema));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id      Int   @id @default(autoincrement())
                user_id Int?  @unique
                User    User? @relation(fields: [user_id], references: [id])
            }
                  
            model User {
                id   Int   @id @default(autoincrement())
                Post Post?
            }       
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_one_to_one_relation_referencing_non_id_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("email varchar(10) UNIQUE");
            });
            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_email varchar(10) UNIQUE");
                t.inject_custom(&format!(
                    "FOREIGN KEY ([user_email]) REFERENCES [{}].[User]([email])",
                    schema
                ));
            });
        })
        .await;
    let dm = r#"
           model Post {
                id         Int     @id @default(autoincrement())
                user_email String? @unique
                User       User?   @relation(fields: [user_email], references: [email])
            }
                  
            model User {
                id    Int     @id @default(autoincrement())
                email String? @unique
                Post  Post?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_one_to_many_relation_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();

    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", move |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("[user_id] INT");
                    t.inject_custom(&format!("FOREIGN KEY ([user_id]) REFERENCES [{}].[User]([id])", schema));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id      Int   @id @default(autoincrement())
                user_id Int?
                User    User? @relation(fields: [user_id], references: [id])
            }
            
            model User {
                id   Int    @id @default(autoincrement())
                Post Post[]
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_one_req_to_many_relation_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", move |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("user_id INT NOT NULL");
                    t.inject_custom(&format!("FOREIGN KEY ([user_id]) REFERENCES [{}].[User]([id])", schema));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id      Int  @id @default(autoincrement())
                user_id Int
                User    User @relation(fields: [user_id], references: [id])
            }
            
            model User {
                id   Int    @id @default(autoincrement())
                Post Post[]
            }
       "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_prisma_many_to_many_relation_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("_PostToUser", move |t| {
                    t.inject_custom("A INT NOT NULL");
                    t.inject_custom("B INT NOT NULL");
                    t.inject_custom(&format!(
                        "FOREIGN KEY ([A]) REFERENCES [{}].[Post]([id]) ON DELETE CASCADE",
                        schema
                    ));
                    t.inject_custom(&format!(
                        "FOREIGN KEY ([B]) REFERENCES [{}].[User]([id]) ON DELETE CASCADE",
                        schema
                    ));
                    t.add_index("test", types::index(vec!["A", "B"]).unique(true));
                    t.add_index("test2", types::index(vec!["B"]).unique(false));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
        model Post {
            id   Int    @id @default(autoincrement())
            User User[]
        }

        model User {
            id   Int    @id @default(autoincrement())
            Post Post[]
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_many_to_many_relation_with_an_id_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("PostsToUsers", move |t| {
                    t.inject_custom("id INT PRIMARY KEY");
                    t.inject_custom("user_id INT NOT NULL");
                    t.inject_custom("post_id INT NOT NULL");
                    t.inject_custom(&format!("FOREIGN KEY ([user_id]) REFERENCES [{}].[User]([id])", schema));
                    t.inject_custom(&format!("FOREIGN KEY ([post_id]) REFERENCES [{}].[Post]([id])", schema));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Post {
                id           Int            @id @default(autoincrement())
                PostsToUsers PostsToUsers[]
            }
            
            model PostsToUsers {
                id      Int  @id
                user_id Int
                post_id Int
                Post    Post @relation(fields: [post_id], references: [id])
                User    User @relation(fields: [user_id], references: [id])
            }
            
            model User {
                id           Int            @id @default(autoincrement())
                PostsToUsers PostsToUsers[]
            }         
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_self_relation_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", move |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("recruited_by INT");
                    t.inject_custom("direct_report INT");
                    t.inject_custom(&format!(
                        "FOREIGN KEY ([recruited_by]) REFERENCES [{}].[User]([id])",
                        schema
                    ));
                    t.inject_custom(&format!(
                        "FOREIGN KEY ([direct_report]) REFERENCES [{}].[User]([id])",
                        schema
                    ));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
              model User {
                id                                  Int    @id @default(autoincrement())
                recruited_by                        Int?
                direct_report                       Int?
                User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id])
                User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id])
                other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_id_fields_with_foreign_key_should_work(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();

    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", move |t| {
                t.add_column("test", types::text());
                t.inject_custom("user_id int primary key");
                t.inject_custom(&format!("FOREIGN KEY ([user_id]) REFERENCES [{}].[User]([id])", schema));
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
                id   Int    @id @default(autoincrement())
                Post Post?
            }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
