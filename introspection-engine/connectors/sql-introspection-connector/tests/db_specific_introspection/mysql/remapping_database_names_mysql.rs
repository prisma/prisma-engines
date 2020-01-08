use crate::*;
use barrel::types;
use test_harness::*;

#[test_one_connector(connector = "mysql")]
async fn remapping_fields_with_invalid_characters_should_work_for_mysql(api: &TestApi) {
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
async fn remapping_tables_with_invalid_characters_should_work_for_mysql(api: &TestApi) {
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
