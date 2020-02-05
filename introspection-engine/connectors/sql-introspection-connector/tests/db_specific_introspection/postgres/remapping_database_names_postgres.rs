use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
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
               a      String @map("_a")
               b      String @map("*b")
               c      String @map("?c")
               d      String @map("(d")
               e      String @map(")e")
               f      String @map("/f")
               g_a    String @map("g a")
               h1     String
               h_a    String @map("h-a")
               id     Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
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
               id Int @id @default(autoincrement())

               @@map("?User")
            }

            model User_with_Space {
               id Int @id @default(autoincrement())

               @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
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
                t.inject_custom("FOREIGN KEY (\"user_id\") REFERENCES \"User with Space\"(\"id\")");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id                  Int                 @id @default(autoincrement())
                user_id     User_with_Space
            }

            model User_with_Space {
               id       Int                             @id @default(autoincrement())
               name     String
               post     Post?

               @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn remapping_models_in_compound_relations_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_name", types::text());
                t.inject_custom(
                    "FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User with Space\"(\"id\", \"name\")",
                );
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\", \"user_name\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                             @id @default(autoincrement())
                user_with_Space    User_with_Space      @map(["user_id", "user_name"]) @relation(references:[id, name])
            }

            model User_with_Space {
               id       Int                             @id @default(autoincrement())
               name     String
               post     Post?

               @@map("User with Space")
               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn remapping_fields_in_compound_relations_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name-that-is-invalid", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name-that-is-invalid\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_name", types::text());
                t.inject_custom(
                    "FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name-that-is-invalid\")",
                );
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\", \"user_name\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id                      Int     @id @default(autoincrement())
                user                    User    @map(["user_id", "user_name"]) @relation(references:[id, name_that_is_invalid])
            }

            model User {
               id                       Int     @id @default(autoincrement())
               name_that_is_invalid     String  @map("name-that-is-invalid")
               post                     Post?

               @@unique([id, name_that_is_invalid], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn remapping_enum_names_should_work(api: &TestApi) {
    let sql1 = format!("CREATE Type _color as ENUM ('black')");
    api.database().execute_raw(&sql1, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  _color Not Null");
            });
        })
        .await;

    let dm = r#"
        model Book {
            color   color
            id      Int     @default(autoincrement()) @id
        }

        enum color {
            black
            @@map("_color")
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn remapping_compound_primary_keys_should_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("first_name", types::text());
                t.add_column("last@name", types::text());
                t.inject_custom("Primary Key (\"first_name\", \"last@name\")");
            });
        })
        .await;

    let dm = r#"
        model User {
            first_name   String
            last_name   String @map("last@name")

            @@id([first_name, last_name])
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
