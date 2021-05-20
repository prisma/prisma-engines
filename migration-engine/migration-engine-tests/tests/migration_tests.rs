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

use migration_engine_tests::{sql::*, TestResult};
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use quaint::prelude::Queryable;
use sql_schema_describer::*;
use test_macros::test_connector;

#[test_connector]
async fn adding_an_id_field_of_type_int_with_autoincrement_works(api: &TestApi) -> TestResult {
    let dm2 = r#"
        model Test {
            myId Int @id @default(autoincrement())
            text String
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("Test", |t| {
        t.assert_column("myId", |c| {
            if api.is_postgres() {
                c.assert_default(Some(DefaultValue::sequence("Test_myId_seq")))
            } else {
                c.assert_auto_increments()
            }
        })
    })?;

    Ok(())
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_connector(exclude(Sqlite))]
async fn making_an_existing_id_field_autoincrement_works(api: &TestApi) -> TestResult {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int        @id
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())
    })?;

    // MySQL cannot add autoincrement property to a column that already has data.
    if !api.sql_family().is_mysql() {
        // Data to see we don't lose anything in the translation.
        for (i, content) in (&["A", "B", "C"]).iter().enumerate() {
            let insert = Insert::single_into(api.render_table_name("Post"))
                .value("content", *content)
                .value("id", i);

            api.database().insert(insert.into()).await?;
        }

        assert_eq!(
            3,
            api.database()
                .select(Select::from_table(api.render_table_name("Post")))
                .await?
                .len()
        );
    }

    let dm2 = r#"
        model Post {
            id        Int         @id @default(autoincrement())
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    // Check that the migration is idempotent.
    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps();

    // MySQL cannot add autoincrement property to a column that already has data.
    if !api.sql_family().is_mysql() {
        assert_eq!(
            3,
            api.database()
                .select(Select::from_table(api.render_table_name("Post")))
                .await?
                .len()
        );
    }

    Ok(())
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_connector(exclude(Sqlite))]
async fn removing_autoincrement_from_an_existing_field_works(api: &TestApi) -> TestResult {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int         @id @default(autoincrement())
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    // Data to see we don't lose anything in the translation.
    for content in &["A", "B", "C"] {
        let insert = Insert::single_into(api.render_table_name("Post")).value("content", *content);
        api.database().insert(insert.into()).await?;
    }

    assert_eq!(
        3,
        api.database()
            .select(Select::from_table(api.render_table_name("Post")))
            .await?
            .len()
    );

    let dm2 = r#"
        model Post {
            id        Int         @id
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())
    })?;

    // Check that the migration is idempotent.
    api.schema_push(dm2)
        .migration_id(Some("idempotency-check"))
        .send()
        .await?
        .assert_green()?
        .assert_no_steps();

    assert_eq!(
        3,
        api.database()
            .select(Select::from_table(api.render_table_name("Post")))
            .await?
            .len()
    );

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn making_an_existing_id_field_autoincrement_works_with_indices(api: &TestApi) -> TestResult {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int        @id
            content   String?

            @@index([content], name: "fooBarIndex")
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model
            .assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())?
            .assert_indexes_count(1)
    })?;

    // Data to see we don't lose anything in the translation.
    for (i, content) in (&["A", "B", "C"]).iter().enumerate() {
        let insert = Insert::single_into(api.render_table_name("Post"))
            .value("content", *content)
            .value("id", i);

        api.database().insert(insert.into()).await?;
    }

    assert_eq!(
        3,
        api.database()
            .select(Select::from_table(api.render_table_name("Post")))
            .await?
            .len()
    );

    let dm2 = r#"
        model Post {
            id        Int         @id @default(autoincrement())
            content   String?

            @@index([content], name: "fooBarIndex")
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model
            .assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())?
            .assert_indexes_count(1)
    })?;

    // Check that the migration is idempotent.
    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps();

    assert_eq!(
        3,
        api.database()
            .select(Select::from_table(api.render_table_name("Post")))
            .await?
            .len()
    );

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn making_an_existing_id_field_autoincrement_works_with_foreign_keys(api: &TestApi) -> TestResult {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int         @id
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
            author_id Int
            author    Author      @relation(fields: [author_id], references: [id])
            trackings Tracking[]
        }

        model Tracking {
            id        Int         @id @default(autoincrement())
            post_id   Int
            post      Post        @relation(fields: [post_id], references: [id])
        }

        model Author {
            id        Int         @id @default(autoincrement())
            posts     Post[]
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())
    })?;

    // Data to see we don't lose anything in the translation.
    for (i, content) in (&["A", "B", "C"]).iter().enumerate() {
        let insert = Insert::single_into(api.render_table_name("Author"));

        let author_id = api
            .database()
            .insert(Insert::from(insert).returning(&["id"]))
            .await?
            .into_single()?
            .into_single()?
            .as_i64()
            .unwrap();

        let insert = Insert::single_into(api.render_table_name("Post"))
            .value("content", *content)
            .value("id", i)
            .value("author_id", author_id);

        api.database().insert(insert.into()).await?;

        let insert = Insert::single_into(api.render_table_name("Tracking")).value("post_id", i);

        api.database().insert(insert.into()).await?;
    }

    assert_eq!(
        3,
        api.database()
            .select(Select::from_table(api.render_table_name("Post")))
            .await?
            .len()
    );

    let dm2 = r#"
        model Post {
            id        Int         @id @default(autoincrement())
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
            author_id Int
            author    Author      @relation(fields: [author_id], references: [id])
            trackings Tracking[]
        }

        model Tracking {
            id        Int         @id @default(autoincrement())
            post_id   Int
            post      Post        @relation(fields: [post_id], references: [id])
        }

        model Author {
            id        Int         @id @default(autoincrement())
            posts     Post[]
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    // TODO: Why not empty?
    // Check that the migration is idempotent.
    //api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps();

    assert_eq!(
        3,
        api.database()
            .select(Select::from_table(api.render_table_name("Post")))
            .await?
            .len()
    );

    Ok(())
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_connector(exclude(Sqlite))]
async fn flipping_autoincrement_on_and_off_works(api: &TestApi) -> TestResult {
    let dm_without = r#"
        model Post {
            id        Int        @id
            title     String     @default("")
        }
    "#;

    let dm_with = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            updatedAt DateTime
        }
    "#;

    api.schema_push(dm_with).send().await?.assert_green()?;
    api.schema_push(dm_without).send().await?.assert_green()?;
    api.schema_push(dm_with).send().await?.assert_green()?;
    api.schema_push(dm_without).send().await?.assert_green()?;
    api.schema_push(dm_with).send().await?.assert_green()?;

    Ok(())
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_connector(exclude(Sqlite))]
async fn making_an_autoincrement_default_an_expression_then_autoincrement_again_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            title     String     @default("")
        }
    "#;

    api.schema_push(dm1)
        .migration_id(Some("apply_dm1"))
        .send()
        .await?
        .assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    let dm2 = r#"
        model Post {
            id        Int       @id @default(3)
            title     String    @default("")
        }
    "#;

    api.schema_push(dm2)
        .migration_id(Some("apply_dm2"))
        .send()
        .await?
        .assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model
            .assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())?
            .assert_column("id", |column| {
                column.assert_default(Some(DefaultValue::value(PrismaValue::Int(3))))
            })
    })?;

    // Now re-apply the sequence.
    api.schema_push(dm1)
        .migration_id(Some("apply_dm1_again"))
        .send()
        .await?
        .assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    Ok(())
}

#[test_connector]
async fn removing_a_scalar_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_columns_count(2)?.assert_has_column("field")
    })?;

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_column_count(1))?;

    Ok(())
}

#[test_connector]
async fn update_type_of_scalar_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_type_is_string())
    })?;

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            field Int
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_type_is_int())
    })?;

    Ok(())
}

#[test_connector]
async fn changing_the_type_of_an_id_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            b_id Int
            b  B   @relation(fields: [b_id], references: [id])
        }

        model B {
            id Int @id
            a  A[]
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::Int))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["id"]))
    })?;

    let dm2 = r#"
        model A {
            id Int @id
            b_id String
            b  B   @relation(fields: [b_id], references: [id])
        }

        model B {
            id String @id @default(cuid())
            a  A[]

        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::String))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["id"]))
    })?;

    Ok(())
}

#[test_connector]
async fn changing_the_type_of_a_field_referenced_by_a_fk_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            b_id Int
            b  B   @relation(fields: [b_id], references: [uniq])
        }

        model B {
            uniq Int @unique
            name String
            a    A[]
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::Int))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["uniq"]))
    })?;

    let dm2 = r#"
        model A {
            id Int @id
            b_id String
            b  B   @relation(fields: [b_id], references: [uniq])
        }

        model B {
            uniq String @unique @default(cuid())
            name String
            a    A[]
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::String))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["uniq"]))
    })?;

    Ok(())
}

#[test_connector]
async fn updating_db_name_of_a_scalar_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id String @id @default(cuid())
            field String @map(name:"name1")
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("A", |table| table.assert_has_column("name1"))?;

    let dm2 = r#"
        model A {
            id String @id @default(cuid())
            field String @map(name:"name2")
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_columns_count(2)?
            .assert_has_column("id")?
            .assert_has_column("name2")
    })?;

    Ok(())
}

#[test_connector]
async fn adding_a_new_unique_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |index| index.assert_is_unique())
    })?;

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

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    })?;

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

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["custom_field_name"], |idx| idx.assert_is_unique())
    })?;

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

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["custom_field_name", "second_custom_field_name"], |idx| {
            idx.assert_is_unique()
        })
    })?;

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

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    })?;

    let dm2 = r#"
        model A {
            id    Int    @id
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_indexes_count(0)?
            .assert_columns_count(1)?
            .assert_has_column("id")
    })?;

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

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("A", |table| table.assert_indexes_count(0))?;

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
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    })?;

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

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |t| {
        t.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    })?;

    let dm2 = r#"
        model A {
            id    Int    @id
            field String
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("A", |t| t.assert_indexes_count(0))?;

    Ok(())
}

// TODO: Enable SQL Server when cascading rules are in PSL.
#[test_connector(exclude(Mssql))]
async fn reserved_sql_key_words_must_work(api: &TestApi) -> TestResult {
    // Group is a reserved keyword
    let dm = r#"
        model Group {
            id          String  @id @default(cuid())
            parent_id   String?
            parent      Group? @relation(name: "ChildGroups", fields: [parent_id], references: id)
            childGroups Group[] @relation(name: "ChildGroups")
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Group", |table| {
        table.assert_fk_on_columns(&["parent_id"], |fk| fk.assert_references("Group", &["id"]))
    })?;

    Ok(())
}

#[test_connector]
async fn migrations_with_many_to_many_related_models_must_not_recreate_indexes(api: &TestApi) -> TestResult {
    // test case for https://github.com/prisma/lift/issues/148
    let dm_1 = r#"
        model User {
            id        String  @id @default(cuid())
            p         Profile[]
        }

        model Profile {
            id        String  @id @default(cuid())
            userId    String
            user      User    @relation(fields: userId, references: id)
            skills    Skill[]
        }

        model Skill {
            id          String  @id @default(cuid())
            profiles    Profile[]
        }
    "#;

    api.schema_push(dm_1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("_ProfileToSkill", |t| {
        t.assert_index_on_columns(&["A", "B"], |idx| idx.assert_is_unique())
    })?;

    let dm_2 = r#"
        model User {
            id        String  @id @default(cuid())
            someField String?
            p         Profile[]
        }

        model Profile {
            id        String  @id @default(cuid())
            userId    String
            user      User    @relation(fields: userId, references: id)
            skills    Skill[]
        }

        model Skill {
            id          String  @id @default(cuid())
            profiles    Profile[]
        }
    "#;

    api.schema_push(dm_2).send().await?;
    api.assert_schema().await?.assert_table("_ProfileToSkill", |table| {
        table.assert_index_on_columns(&["A", "B"], |idx| {
            idx.assert_is_unique()?.assert_name("_ProfileToSkill_AB_unique")
        })
    })?;

    Ok(())
}

#[test_connector]
async fn removing_a_relation_field_must_work(api: &TestApi) -> TestResult {
    let dm_1 = r#"
        model User {
            id        String  @id @default(cuid())
            address_id String @map("address_name")
            address   Address @relation(fields: [address_id], references: [id])
        }

        model Address {
            id        String  @id @default(cuid())
            street    String
            u         User[]
        }
    "#;

    api.schema_push(dm_1).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("User", |table| table.assert_has_column("address_name"))?;

    let dm_2 = r#"
        model User {
            id        String  @id @default(cuid())
        }

        model Address {
            id        String  @id @default(cuid())
            street    String
        }
    "#;

    api.schema_push(dm_2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("User", |table| table.assert_does_not_have_column("address_name"))?;

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

    api.schema_push(dm1).send().await?.assert_green()?;

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
    })?;

    let insert = Insert::single_into(api.render_table_name("Fruit")).value("name", "banana");
    api.database().query(insert.into()).await.unwrap();

    let dm2 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps();

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn renaming_a_datasource_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        datasource db1 {
            provider = "sqlite"
            url = "file:///tmp/prisma-test.db"
        }

        model User {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let dm2 = r#"
        datasource db2 {
            provider = "sqlite"
            url = "file:///tmp/prisma-test.db"
        }

        model User {
            id Int @id
        }
    "#;

    api.schema_push(dm2).migration_id(Some("mig02")).send().await?;

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

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["lastName", "firstName"]))
    })?;

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

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["first_name", "family_name"]))
    })?;

    Ok(())
}

#[test_connector]
async fn references_to_models_with_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            firstName String
            lastName  String
            pets      Pet[]

            @@id([firstName, lastName])
        }

        model Pet {
            id              String @id
            human_firstName String
            human_lastName  String

            human User @relation(fields: [human_firstName, human_lastName], references: [firstName, lastName])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Pet", |table| {
        table
            .assert_has_column("id")?
            .assert_has_column("human_firstName")?
            .assert_has_column("human_lastName")?
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
                fk.assert_references("User", &["firstName", "lastName"])
            })
    })?;

    Ok(())
}

#[test_connector]
async fn join_tables_between_models_with_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Human {
            firstName String
            lastName String
            cats HumanToCat[]

            @@id([firstName, lastName])
        }

        model HumanToCat {
            human_firstName String
            human_lastName String
            cat_id String

            cat Cat @relation(fields: [cat_id], references: [id], onDelete: Cascade)
            human Human @relation(fields: [human_firstName, human_lastName], references: [firstName, lastName], onDelete: Cascade)

            @@unique([cat_id, human_firstName, human_lastName], name: "joinTableUnique")
            @@index([human_firstName, human_lastName], name: "joinTableIndex")
        }

        model Cat {
            id String @id
            humans HumanToCat[]
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("HumanToCat", |table| {
        table
            .assert_has_column("human_firstName")?
            .assert_has_column("human_lastName")?
            .assert_has_column("cat_id")?
            .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
                fk.assert_references("Human", &["firstName", "lastName"])?
                    .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
            })?
            .assert_fk_on_columns(&["cat_id"], |fk| {
                fk.assert_references("Cat", &["id"])?
                    .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
            })?
            .assert_indexes_count(2)?
            .assert_index_on_columns(&["cat_id", "human_firstName", "human_lastName"], |idx| {
                idx.assert_is_unique()
            })?
            .assert_index_on_columns(&["human_firstName", "human_lastName"], |idx| idx.assert_is_not_unique())
    })?;

    Ok(())
}

#[test_connector]
async fn join_tables_between_models_with_mapped_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Human {
            firstName String @map("the_first_name")
            lastName String @map("the_last_name")
            cats HumanToCat[]

            @@id([firstName, lastName])
        }

        model HumanToCat {
            human_the_first_name String
            human_the_last_name String
            cat_id String

            cat Cat @relation(fields: [cat_id], references: [id])
            human Human @relation(fields: [human_the_first_name, human_the_last_name], references: [firstName, lastName])

            @@unique([human_the_first_name, human_the_last_name, cat_id], name: "joinTableUnique")
            @@index([cat_id])
        }

        model Cat {
            id String @id
            humans HumanToCat[]
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("HumanToCat", |table| {
        table
            .assert_has_column("human_the_first_name")?
            .assert_has_column("human_the_last_name")?
            .assert_has_column("cat_id")?
            .assert_fk_on_columns(&["human_the_first_name", "human_the_last_name"], |fk| {
                fk.assert_references("Human", &["the_first_name", "the_last_name"])
            })?
            .assert_fk_on_columns(&["cat_id"], |fk| fk.assert_references("Cat", &["id"]))?
            .assert_indexes_count(2)
    })?;

    Ok(())
}

#[test_connector]
async fn switching_databases_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model Test {
            id String @id
            name String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let dm2 = r#"
        datasource db {
            provider = "sqlite"
            url = "file:hiya.db"
        }

        model Test {
            id String @id
            name String
        }
    "#;

    api.schema_push(dm2)
        .migration_id(Some("mig2"))
        .send()
        .await?
        .assert_green()?;

    Ok(())
}

// TODO: Enable SQL Server when cascading rules are in PSL.
#[test_connector(exclude(Mssql))]
async fn adding_mutual_references_on_existing_tables_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
        }

        model B {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let dm2 = r#"
        model A {
            id Int
            name String @unique
            b_email String
            brel B @relation("AtoB", fields: [b_email], references: [email])
            b    B[] @relation("BtoA")
        }

        model B {
            id Int
            email String @unique
            a_name String
            arel A @relation("BtoA", fields: [a_name], references: [name])
            a    A[] @relation("AtoB")
        }
    "#;

    let res = api.schema_push(dm2).force(true).send().await?;

    if api.sql_family().is_sqlite() {
        res.assert_green()?;
    } else {
        res.assert_warnings(&["A unique constraint covering the columns `[name]` on the table `A` will be added. If there are existing duplicate values, this will fail.".into(), "A unique constraint covering the columns `[email]` on the table `B` will be added. If there are existing duplicate values, this will fail.".into()]);
    };

    Ok(())
}

#[test_connector]
async fn models_with_an_autoincrement_field_as_part_of_a_multi_field_id_can_be_created(api: &TestApi) -> TestResult {
    let dm = r#"
        model List {
            id        Int  @id @default(autoincrement())
            uList     String? @unique
            todoId    Int @default(1)
            todoName  String
            todo      Todo   @relation(fields: [todoId, todoName], references: [id, uTodo])
        }

        model Todo {
            id     Int @default(autoincrement())
            uTodo  String
            lists  List[]

            @@id([id, uTodo])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Todo", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id", "uTodo"]))?
            .assert_column("id", |col| {
                if api.is_sqlite() {
                    Ok(col)
                } else {
                    col.assert_auto_increments()
                }
            })
    })?;

    Ok(())
}

#[test_connector]
async fn migrating_a_unique_constraint_to_a_primary_key_works(api: &TestApi) -> TestResult {
    let dm = r#"
        model model1 {
            id              String        @id @default(cuid())
            a               String
            b               String
            c               String

            @@unique([a, b, c])

        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("model1", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id"]))?
            .assert_index_on_columns(&["a", "b", "c"], |idx| idx.assert_is_unique())
    })?;

    api.insert("model1")
        .value("id", "the-id")
        .value("a", "the-a")
        .value("b", "the-b")
        .value("c", "the-c")
        .result_raw()
        .await?;

    let dm2 = r#"
        model model1 {
            a               String
            b               String
            c               String

            @@id([a, b, c])

        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send()
        .await?
        .assert_executable()
        .assert_warnings(&["The primary key for the `model1` table will be changed. If it partially fails, the table could be left without primary key constraint.".into(), "You are about to drop the column `id` on the `model1` table, which still contains 1 non-null values.".into()]);

    api.assert_schema().await?.assert_table("model1", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["a", "b", "c"]))
    })?;

    Ok(())
}

#[test_connector]
async fn adding_multiple_optional_fields_to_an_existing_model_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let dm2 = r#"
        model Cat {
            id   Int @id
            name String?
            age  Int?
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table
            .assert_column("name", |col| col.assert_is_nullable())?
            .assert_column("age", |col| col.assert_is_nullable())
    })?;

    Ok(())
}

// TODO: Enable SQL Server when cascading rules are in PSL.
#[test_connector(exclude(Mssql))]
async fn reordering_and_altering_models_at_the_same_time_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            name Int @unique
            c C @relation(name: "atoc", fields: [name], references: [name])
            cs C[] @relation(name: "ctoa")
        }

        model B {
            id Int @id
            name Int @unique
            c C @relation(name: "btoc", fields: [name], references: [name])
        }

        model C {
            id Int @id
            name Int @unique
            a A @relation(name: "ctoa", fields: [name], references: [name])
            as A[] @relation(name: "atoc")
            bs B[] @relation(name: "btoc")
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let dm2 = r#"
        model C {
            id Int @id
            a A @relation(name: "ctoa2", fields: [name], references: [name])
            name Int @unique
            bs B[] @relation(name: "btoc2")
            as A[] @relation(name: "atoc2")
        }

        model A {
            id Int @id
            name Int @unique
            c C @relation(name: "atoc2", fields: [name], references: [name])
            cs C[] @relation(name: "ctoa2")
        }

        model B {
            c C @relation(name: "btoc2", fields: [name], references: [name])
            name Int @unique
            id Int @id
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn a_table_recreation_with_noncastable_columns_should_trigger_warnings(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Blog {
            id Int @id @default(autoincrement())
            title String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    // Removing autoincrement requires us to recreate the table.
    let dm2 = r#"
        model Blog {
            id Int @id
            title Float
        }
    "#;

    api.insert("Blog").value("title", "3.14").result_raw().await?;

    api.schema_push(dm2)
        .send()
        .await?
        .assert_warnings(&["You are about to alter the column `title` on the `Blog` table, which contains 1 non-null values. The data in that column will be cast from `String` to `Float`.".into()]);

    Ok(())
}

// #[test_connector]
// //todo something is rotten in the state of Denmark
// async fn a_column_recreation_with_non_castable_type_change_should_trigger_warnings(api: &TestApi) -> TestResult {
//     let dm1 = r#"
//         model Blog {
//             id      Int @id
//             float   Float
//         }
//     "#;
//
//     api.schema_push(dm1).send().await?.assert_green()?;
//     let insert = Insert::single_into((api.schema_name(), "Blog"))
//         .value("id", 1)
//         .value("float", Value::double(7.5));
//
//     api.database().insert(insert.into()).await?;
//     let dm2 = r#"
//         model Blog {
//             id      Int @id
//             float   DateTime
//         }
//     "#;
//
//     api.schema_push(dm2).send().await?;
//     // .assert_warnings(&["You are about to alter the column `float` on the `Blog` table. The data in that column will be cast from `String` to `Float`. This cast may fail and the migration will stop. Please make sure the data in the column can be cast.".into()])?;
//
//     Ok(())
// }
