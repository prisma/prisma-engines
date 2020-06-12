use crate::*;
use crate::{custom_assert, test_each_connector, TestApi};
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_mapped_model_name(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model Custom_User {
               id               Int @id @default(autoincrement())
               
               @@map(name: "User")
            }
        "#;

    let final_dm = r#"
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
            
            model Custom_User {
               id               Int @id @default(autoincrement())
               
               @@map(name: "User")
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_mapped_field_name(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model User {
               custom_id               Int @id @default(autoincrement()) @map("id")
            }
        "#;

    let final_dm = r#"
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
            
            model User {
               custom_id               Int @id @default(autoincrement()) @map("id")
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_custom_virtual_relation_field_names(api: &TestApi) {
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
            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
             model Post {
               id               Int @id @default(autoincrement())
               user_id          Int  @unique
               custom_User      User @relation(fields: [user_id], references: [id])
            }

            model User {
               id               Int @id @default(autoincrement())
               custom_Post      Post?
            }
        "#;

    let final_dm = r#"
             model Post {
               id               Int @id @default(autoincrement())
               user_id          Int  @unique
               custom_User      User @relation(fields: [user_id], references: [id])
            }

            model Unrelated {
               id               Int @id @default(autoincrement())
            }
            
            model User {
               id               Int @id @default(autoincrement())
               custom_Post      Post?
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

// #[test_each_connector(tags("postgres"))]
// async fn re_introspecting_virtual_default(api: &TestApi) {
//     let barrel = api.barrel();
//     let _setup_schema = barrel
//         .execute(|migration| {
//             migration.create_table("User", |t| {
//                 t.add_column("id", types::primary());
//                 t.add_column("text", types::text());
//             });
//             migration.create_table("Unrelated", |t| {
//                 t.add_column("id", types::primary());
//             });
//         })
//         .await;
//
//     let input_dm = r#"
//             model User {
//                id        Int    @id @default(autoincrement())
//                text      String @default("virtual_default")
//             }
//         "#;
//
//     let final_dm = r#"
//             model Unrelated {
//                id               Int @id @default(autoincrement())
//             }
//
//              model User {
//                id        Int    @id @default(autoincrement())
//                text      String @default("virtual_default")
//             }
//         "#;
//     let result = dbg!(api.re_introspect(input_dm).await);
//     custom_assert(&result, final_dm);
// }
