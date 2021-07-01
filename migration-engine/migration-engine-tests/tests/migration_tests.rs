mod apply_migrations;
mod create_migration;
mod errors;
mod evaluate_data_loss;
mod existing_data;
mod initialization;
mod list_migration_directories;
mod migrations;
mod native_types;
mod schema_push;

use migration_engine_tests::sql::*;
use sql_schema_describer::*;
use test_macros::test_connector;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

#[test_connector]
async fn adding_a_new_unique_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();

    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |index| index.assert_is_unique())
    });

    Ok(())
}

#[test_connector]
async fn adding_new_fields_with_multi_column_unique_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField String

            @@unique([field, secondField])
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    Ok(())
}

#[test_connector]
async fn unique_in_conjunction_with_custom_column_name_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique @map("custom_field_name")
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["custom_field_name"], |idx| idx.assert_is_unique())
    });

    Ok(())
}

#[test_connector]
async fn multi_column_unique_in_conjunction_with_custom_column_name_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String @map("custom_field_name")
            secondField String @map("second_custom_field_name")

            @@unique([field, secondField])
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["custom_field_name", "second_custom_field_name"], |idx| {
            idx.assert_is_unique()
        })
    });

    Ok(())
}

#[test_connector]
async fn removing_an_existing_unique_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String @unique
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();
    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green_bang();

    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_indexes_count(0)
            .assert_columns_count(1)
            .assert_has_column("id")
    });

    Ok(())
}

#[test_connector]
async fn adding_unique_to_an_existing_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();

    api.assert_schema()
        .await?
        .assert_table("A", |table| table.assert_indexes_count(0));

    let dm2 = r#"
        model A {
            id    Int    @id
            field String @unique
        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send().await?
        .assert_executable()
        .assert_warnings(&["A unique constraint covering the columns `[field]` on the table `A` will be added. If there are existing duplicate values, this will fail.".into()])
        .assert_has_executed_steps();

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_indexes_count(1)
            .assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });

    Ok(())
}

#[test_connector]
async fn removing_unique_from_an_existing_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String @unique
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
            field String
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green_bang();
    api.assert_schema()
        .await?
        .assert_table("A", |t| t.assert_indexes_count(0));

    Ok(())
}

#[test_connector]
async fn simple_type_aliases_in_migrations_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        type CUID = String @id @default(cuid())

        model User {
            id CUID
            age Float
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green_bang();

    Ok(())
}

#[test_connector]
async fn created_at_does_not_get_arbitrarily_migrated(api: &TestApi) -> TestResult {
    use quaint::ast::Insert;

    let dm1 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    api.schema_push(dm1).send().await?;
    api.assert_schema().await?.assert_table("Fruit", |t| {
        t.assert_column("createdAt", |c| c.assert_default(Some(DefaultValue::now())))
    });

    let insert = Insert::single_into(api.render_table_name("Fruit")).value("name", "banana");
    api.database().query(insert.into()).await.unwrap();

    let dm2 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green_bang().assert_no_steps();

    Ok(())
}

#[test_connector]
async fn basic_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            firstName String
            lastName String

            @@id([lastName, firstName])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green_bang();

    api.assert_schema().await?.assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["lastName", "firstName"]))
    });

    Ok(())
}

#[test_connector]
async fn compound_primary_keys_on_mapped_columns_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            firstName String @map("first_name")
            lastName String @map("family_name")

            @@id([firstName, lastName])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green_bang();

    api.assert_schema().await?.assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["first_name", "family_name"]))
    });

    Ok(())
}
