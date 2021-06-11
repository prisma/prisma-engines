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
use quaint::prelude::Queryable;
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

    api.schema_push(dm_1).send().await?.assert_green_bang();
    api.assert_schema().await?.assert_table("_ProfileToSkill", |t| {
        t.assert_index_on_columns(&["A", "B"], |idx| idx.assert_is_unique())
    });

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
            idx.assert_is_unique().assert_name("_ProfileToSkill_AB_unique")
        })
    });

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

    api.schema_push(dm_1).send().await?.assert_green_bang();

    api.assert_schema()
        .await?
        .assert_table("User", |table| table.assert_has_column("address_name"));

    let dm_2 = r#"
        model User {
            id        String  @id @default(cuid())
        }

        model Address {
            id        String  @id @default(cuid())
            street    String
        }
    "#;

    api.schema_push(dm_2).send().await?.assert_green_bang();

    api.assert_schema()
        .await?
        .assert_table("User", |table| table.assert_does_not_have_column("address_name"));

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

    api.schema_push(dm1).send().await?.assert_green_bang();

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

    api.schema_push(dm).send().await?.assert_green_bang();

    api.assert_schema().await?.assert_table("Pet", |table| {
        table
            .assert_has_column("id")
            .assert_has_column("human_firstName")
            .assert_has_column("human_lastName")
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
                fk.assert_references("User", &["firstName", "lastName"])
            })
    });

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

            cat Cat @relation(fields: [cat_id], references: [id])
            human Human @relation(fields: [human_firstName, human_lastName], references: [firstName, lastName])

            @@unique([cat_id, human_firstName, human_lastName], name: "joinTableUnique")
            @@index([human_firstName, human_lastName], name: "joinTableIndex")
        }

        model Cat {
            id String @id
            humans HumanToCat[]
        }
    "#;

    api.schema_push(dm).send().await?.assert_green_bang();

    api.assert_schema().await?.assert_table("HumanToCat", |table| {
        table
            .assert_has_column("human_firstName")
            .assert_has_column("human_lastName")
            .assert_has_column("cat_id")
            .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
                fk.assert_references("Human", &["firstName", "lastName"])
                    .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
            })
            .assert_fk_on_columns(&["cat_id"], |fk| {
                fk.assert_references("Cat", &["id"])
                    .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
            })
            .assert_indexes_count(2)
            .assert_index_on_columns(&["cat_id", "human_firstName", "human_lastName"], |idx| {
                idx.assert_is_unique()
            })
            .assert_index_on_columns(&["human_firstName", "human_lastName"], |idx| idx.assert_is_not_unique())
    });

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

    api.schema_push(dm).send().await?.assert_green_bang();

    api.assert_schema().await?.assert_table("HumanToCat", |table| {
        table
            .assert_has_column("human_the_first_name")
            .assert_has_column("human_the_last_name")
            .assert_has_column("cat_id")
            .assert_fk_on_columns(&["human_the_first_name", "human_the_last_name"], |fk| {
                fk.assert_references("Human", &["the_first_name", "the_last_name"])
            })
            .assert_fk_on_columns(&["cat_id"], |fk| fk.assert_references("Cat", &["id"]))
            .assert_indexes_count(2)
    });

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

    api.schema_push(dm1).send().await?.assert_green_bang();

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
        .assert_green_bang();

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

    api.schema_push(dm1).send().await?.assert_green_bang();

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

    api.schema_push(dm2).send().await?.assert_green_bang();

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

    api.schema_push(dm1).send().await?.assert_green_bang();

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
//     api.schema_push(dm1).send().await?.assert_green_bang();
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
