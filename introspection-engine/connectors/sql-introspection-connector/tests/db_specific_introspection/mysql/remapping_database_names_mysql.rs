use crate::*;
use barrel::types;
use test_harness::*;

#[test_one_connector(connector = "mysql")]
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
               id     Int @id
               a      String @map("_a")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
               id      Int @id

               @@map("?User")
            }

            model User_with_Space {
               id      Int @id

               @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
                id                  Int                 @id 
                user_id     User_with_Space
            }

            model User_with_Space {
               id       Int                             @id 
               name     String
               post     Post?
               
               @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User with Space`(`id`, `age`)");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(`user_id`, `user_age`)");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                             @id
                user_with_Space    User_with_Space      @map(["user_id", "user_age"]) @relation(references:[id, age]) 
            }

            model User_with_Space {
               age      Int
               id       Int                             @id 
               post     Post?
               
               @@map("User with Space")
               @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
                id                      Int     @id
                user                    User    @map(["user_id", "user_age"]) @relation(references:[id, age_that_is_invalid]) 
            }

            model User {
               age_that_is_invalid      Int     @map("age-that-is-invalid")
               id                       Int     @id
               post                     Post?
               
               @@unique([id, age_that_is_invalid], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
