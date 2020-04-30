use crate::*;
use barrel::types;
use introspection_connector::Version;
use test_harness::*;

#[test_each_connector(tags("sqlite"))]
async fn introspect_sqlite_prisma2(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_Migration", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma2);
}

#[test_each_connector(tags("sqlite"))]
async fn introspect_sqlite_non_prisma(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::NonPrisma);
}

// #[test_each_connector(tags("mysql"))]
// async fn introspecting_a_table_enums_should_work(api: &TestApi) {}

// #[test_each_connector(tags("postgresql"))]
// async fn introspecting_a_table_with_an_enum_default_value_that_is_an_empty_string_should_work(api: &TestApi) {
//     api.barrel()
//         .execute(|migration| {
//             migration.create_table("Book", |t| {
//                 t.add_column("id", types::primary());
//                 t.inject_custom("color  ENUM ( 'black', '') Not Null default ''");
//             });
//         })
//         .await;
//
//     let dm = r#"
//         model Book {
//             color   Book_color  @default(EMPTY_ENUM_VALUE)
//             id      Int         @default(autoincrement()) @id
//         }
//
//         enum Book_color{
//             black
//             EMPTY_ENUM_VALUE    @map("")
//         }
//
//     "#;
//
//     let result = dbg!(api.introspect().await);
//     custom_assert(&result, dm);
// }
