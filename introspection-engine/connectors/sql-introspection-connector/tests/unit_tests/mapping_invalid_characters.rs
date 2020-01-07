use crate::*;
use barrel::types;
use test_harness::*;

async fn setup_invalid_fields(api: &TestApi) {
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
}

#[test_one_connector(connector = "postgres")]
async fn fields_with_invalid_characters_should_be_mapped_for_postgres(api: &TestApi) {
    setup_invalid_fields(api).await;

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
               id     Int @id @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn fields_with_invalid_characters_should_be_mapped_for_mysql(api: &TestApi) {
    setup_invalid_fields(api).await;

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

#[test_one_connector(connector = "sqlite")]
async fn fields_with_invalid_characters_should_be_mapped_for_sqlite(api: &TestApi) {
    setup_invalid_fields(api).await;

    let dm = r#"
            model User {
               d      String @map("(d")
               e      String @map(")e")
               b      String @map("*b")
               f      String @map("/f")
               c      String @map("?c")
               a      String @map("_a")
               g_a    String @map("g a")
               h_a    String @map("h-a")
               h1     String
               id     Int @id
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

async fn setup_invalid_models(api: &TestApi) {
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
}

#[test_one_connector(connector = "postgres")]
async fn tables_with_invalid_characters_should_be_mapped_for_postgres(api: &TestApi) {
    setup_invalid_models(api).await;

    let dm = r#"
            model User {
               id Int @id @sequence(name: "?User_id_seq", allocationSize: 1, initialValue: 1)

               @@map("?User")
            }

            model User_with_Space {
               id Int @id @sequence(name: "User with Space_id_seq", allocationSize: 1, initialValue: 1)

               @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
async fn tables_with_invalid_characters_should_be_mapped_for_mysql(api: &TestApi) {
    setup_invalid_models(api).await;

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

#[test_one_connector(connector = "sqlite")]
async fn tables_with_invalid_characters_should_be_mapped_for_sqlite(api: &TestApi) {
    setup_invalid_models(api).await;

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

#[test_one_connector(connector = "postgres")]
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
                t.inject_custom("FOREIGN KEY (\"user_id\") REFERENCES \"User\"(\"id\")");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id                  Int                 @id @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
                user_with_Space     User_with_Space
            }

            model User_with_Space {
               id       Int                             @id @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               name     String
               post     Post?
               
               @@map("User with Space")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//todo
// fields in relations
