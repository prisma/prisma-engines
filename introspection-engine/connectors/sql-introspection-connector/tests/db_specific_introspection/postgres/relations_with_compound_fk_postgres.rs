use crate::*;
use barrel::types;
use test_harness::*;

//todo compound foreign keys
// more test cases:
// to one relations, required relations, to many relations
// separate uniques on compound fields
// default values on compound fields
// field / model names that need sanitizing

#[test_one_connector(connector = "postgres")]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_one_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_name", types::text());
                t.inject_custom("FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\", \"user_name\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                 @id @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
                user    User                @map(["user_id", "user_name"]) @relation(references:[id, name]) 
            }

            model User {
               id       Int                 @id @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               name     String
               post     Post?
               
               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "postgres")]
#[test]
async fn compound_foreign_keys_should_work_for_required_one_to_one_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("user_name", types::text().nullable(false));
                t.inject_custom("FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\", \"user_name\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                 @id @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
                user    User                @map(["user_id", "user_name"]) @relation(references:[id, name]) 
            }

            model User {
               id       Int                 @id @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               name     String
               post     Post
               
               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "postgres")]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_many_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_name", types::text());
                t.inject_custom("FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                 @id @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
                user    User                @map(["user_id", "user_name"]) @relation(references:[id, name]) 
            }

            model User {
               id       Int                 @id @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               name     String
               posts     Post[]
               
               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
