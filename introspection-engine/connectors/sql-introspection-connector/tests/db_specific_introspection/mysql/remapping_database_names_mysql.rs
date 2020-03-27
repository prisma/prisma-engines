use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("mysql"))]
async fn remapping_fields_with_invalid_characters_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("_a", types::text());
                t.add_column("*b", types::text());
                t.add_column("?c", types::text());
                t.add_column("(d", types::text());
                t.add_column(")e", types::text());
                t.add_column("/f", types::text());
                t.add_column("g a", types::text());
                t.add_column("h-a", types::text());
                t.add_column("h1", types::text());
            });
        })
        .await;
    let dm = r#"
            model User {
               d      String @map("(d")
               e      String @map(")e")
               b      String @map("*b")
               f      String @map("/f")
               c      String @map("?c")
               g_a    String @map("g a")
               h_a    String @map("h-a")
               h1     String
               id     Int @id @default(autoincrement())
               a      String @map("_a")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn remapping_tables_with_invalid_characters_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("?User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;
    let dm = r#"
            model User {
               id      Int @id @default(autoincrement())

               @@map("?User")
            }

            model User_with_Space {
               id      Int @id @default(autoincrement())

               @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn remapping_models_in_relations_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.inject_custom("FOREIGN KEY (`user_id`) REFERENCES `User with Space`(`id`)");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(`user_id`)");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id              Int             @default(autoincrement()) @id
                user_id         Int             @unique
                User_with_Space User_with_Space @relation(fields: [user_id], references: [id])
            }
                  
            model User_with_Space {
                id   Int    @default(autoincrement()) @id
                name String
                Post Post?
                        
                @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn remapping_models_in_relations_should_not_map_virtual_fields(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
            });
            migration.create_table("Post With Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.inject_custom("FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(`user_id`)");
            });
        })
        .await;

    let dm = r#"
            model Post_With_Space {
                id      Int  @default(autoincrement()) @id
                user_id Int  @unique
                User    User @relation(fields: [user_id], references: [id])
                
                @@map("Post With Space")
            }
            
            model User {
                id              Int              @default(autoincrement()) @id
                name            String
                Post_With_Space Post_With_Space?
            }          
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn remapping_models_in_compound_relations_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age`)");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom(
                    "FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User with Space`(`id`, `age`)",
                );
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(`user_id`, `user_age`)");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id              Int             @default(autoincrement()) @id
                user_age        Int
                user_id         Int
                User_with_Space User_with_Space @relation(fields: [user_id, user_age], references: [id, age])
                    
                @@unique([user_id, user_age], name: "post_user_unique")
            }
                      
            model User_with_Space {
                age  Int
                id   Int   @default(autoincrement()) @id
                Post Post?
                            
                @@map("User with Space")
                @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn remapping_fields_in_compound_relations_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age-that-is-invalid", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age-that-is-invalid`)");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User`(`id`, `age-that-is-invalid`)");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(`user_id`, `user_age`)");
            });
        })
        .await;

    let dm = r#" 
            model Post {
                id       Int  @default(autoincrement()) @id
                user_age Int
                user_id  Int
                User     User @relation(fields: [user_id, user_age], references: [id, age_that_is_invalid])
                    
                @@unique([user_id, user_age], name: "post_user_unique")
            }
                      
            model User {
                age_that_is_invalid Int   @map("age-that-is-invalid")
                id                  Int   @default(autoincrement()) @id
                Post                Post?
                            
                @@unique([id, age_that_is_invalid], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn remapping_enum_names_should_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("123MySQLBook", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("1color  Enum('black')");
            });
        })
        .await;

    let dm = r#"
        model MySQLBook {
            color   MySQLBook_color? @map("1color")
            id      Int     @default(autoincrement()) @id
            @@map("123MySQLBook")
        }

        enum MySQLBook_color {
            black
            @@map("123MySQLBook_1color")
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn remapping_enum_values_should_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  Enum('b lack', 'w hite')");
            });
        })
        .await;

    let dm = r#"
        model Book {
            color   Book_color?
            id      Int     @default(autoincrement()) @id
        }

        enum Book_color {
            b_lack   @map("b lack")
            w_hite   @map("w hite")
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn remapping_enum_default_values_should_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  Enum(\"b lack\", \"white\") Not Null default \"b lack\"");
            });
        })
        .await;

    let dm = r#"
        model Book {
            color   Book_color   @default(b_lack)
            id      Int     @default(autoincrement()) @id
        }

        enum Book_color{
            b_lack @map("b lack")
            white
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
