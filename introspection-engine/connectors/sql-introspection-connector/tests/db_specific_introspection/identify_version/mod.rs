use crate::*;
use barrel::types;
use test_harness::*;

// Enum (NON_PRISMA, PRISMA_1, PRISMA_1_1, PRISMA_2)

//Testing
// SQLITE
// -> PRISMA_2: Migration Table, no-onDelete, no non-default types (D)
// -> NON_PRISMA: fallthrough

// MYSQL
// -> PRISMA_2: Migration Table, no-onDelete, no non-default types (D)
// -> PRISMA_1: No Migration Table, no-onDelete, all relation tables with Id, createdAt/updatedAt always exist, no non-default types (D)
// -> PRISMA_1_1: No Migration Table, no-onDelete, all relation tables without Id, no non-default types (D)
// -> NON_PRISMA: fall through

// POSTGRES
// -> PRISMA_2: Migration Table, no-onDelete, no non-default types (D)
// -> PRISMA_1: No Migration Table, no-onDelete, all relation tables with Id, createdAt/updatedAt always exist, no non-default types (D)
// -> PRISMA_1_1: No Migration Table, no-onDelete, all relation tables without Id, no non-default types (D)
// -> NON_PRISMA: fall through

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_enums_should_work(api: &TestApi) {}

#[test_each_connector(tags("postgresql"))]
async fn introspecting_a_table_with_an_enum_default_value_that_is_an_empty_string_should_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  ENUM ( 'black', '') Not Null default ''");
            });
        })
        .await;

    let dm = r#"
        model Book {
            color   Book_color  @default(EMPTY_ENUM_VALUE)
            id      Int         @default(autoincrement()) @id
        }

        enum Book_color{
            black
            EMPTY_ENUM_VALUE    @map("")
        }

    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_enums_should_return_alphabetically_even_when_in_different_order(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect().await);
    custom_assert(&result4, dm);
}
