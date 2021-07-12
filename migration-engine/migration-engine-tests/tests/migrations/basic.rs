use migration_engine_tests::sync_test_api::*;
use sql_schema_describer::{ColumnTypeFamily, DefaultValue};

#[test_connector]
fn adding_an_id_field_of_type_int_with_autoincrement_works(api: TestApi) {
    let dm2 = r#"
        model Test {
            myId Int @id @default(autoincrement())
            text String
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();
    api.assert_schema().assert_table("Test", |t| {
        t.assert_column("myId", |c| {
            if api.is_postgres() {
                c.assert_default(Some(DefaultValue::sequence("Test_myId_seq")))
            } else {
                c.assert_auto_increments()
            }
        })
    });
}

#[test_connector]
fn adding_multiple_optional_fields_to_an_existing_model_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();

    let dm2 = r#"
        model Cat {
            id   Int @id
            name String?
            age  Int?
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();

    api.assert_schema().assert_table("Cat", |table| {
        table
            .assert_column("name", |col| col.assert_is_nullable())
            .assert_column("age", |col| col.assert_is_nullable())
    });
}

#[test_connector]
fn a_model_can_be_removed(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = r#"
        model User {
            id   Int     @id @default(autoincrement())
            name String?
            Post Post[]
        }

        model Post {
            id     Int    @id @default(autoincrement())
            title  String
            User   User   @relation(fields: [userId], references: [id])
            userId Int
        }
    "#;

    api.create_migration("initial", dm1, &directory).send_sync();

    let dm2 = r#"
        model User {
            id   Int     @id @default(autoincrement())
            name String?
        }
    "#;

    api.create_migration("second-migration", dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let output = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(output.is_empty());
}

#[test_connector]
fn adding_a_scalar_field_must_work(api: TestApi) {
    let dm = format!(
        r#"
        {}

        model Test {{
            id          String @id @default(cuid())
            int         Int
            bigInt      BigInt
            float       Float
            boolean     Boolean
            string      String
            dateTime    DateTime
            decimal     Decimal
            bytes       Bytes
        }}
    "#,
        api.datasource_block(),
    );

    api.schema_push(&dm).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table
            .assert_columns_count(9)
            .assert_column("int", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("bigInt", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::BigInt)
            })
            .assert_column("float", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Float)
            })
            .assert_column("boolean", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Boolean)
            })
            .assert_column("string", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::String)
            })
            .assert_column("dateTime", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::DateTime)
            })
            .assert_column("decimal", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Decimal)
            })
            .assert_column("bytes", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Binary)
            })
    });

    // Check that the migration is idempotent.
    api.schema_push(dm).send().assert_green_bang().assert_no_steps();
}

#[test_connector]
fn adding_an_optional_field_must_work(api: TestApi) {
    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            field String?
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();
    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_default(None).assert_is_nullable())
    });
}

#[test_connector]
fn adding_an_id_field_with_a_special_name_must_work(api: TestApi) {
    let dm2 = r#"
        model Test {
            specialName String @id @default(cuid())
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();
    api.assert_schema()
        .assert_table("Test", |table| table.assert_has_column("specialName"));
}

#[test_connector(exclude(Sqlite))]
fn adding_an_id_field_of_type_int_must_work(api: TestApi) {
    let dm2 = r#"
        model Test {
            myId Int @id
            text String
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();
    api.assert_schema()
        .assert_table("Test", |t| t.assert_column("myId", |c| c.assert_no_auto_increment()));
}

#[test_connector(tags(Sqlite))]
fn adding_an_id_field_of_type_int_must_work_for_sqlite(api: TestApi) {
    let dm2 = r#"
        model Test {
            myId Int @id
            text String
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("myId", |col| col.assert_auto_increments())
    });
}

#[test_connector]
fn removing_a_scalar_field_must_work(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();

    api.assert_schema()
        .assert_table("Test", |table| table.assert_columns_count(2).assert_has_column("field"));

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();

    api.assert_schema()
        .assert_table("Test", |table| table.assert_column_count(1));
}

#[test_connector]
fn update_type_of_scalar_field_must_work(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_type_is_string())
    });

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            field Int
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_type_is_int())
    });
}

#[test_connector]
fn updating_db_name_of_a_scalar_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id String @id @default(cuid())
            field String @map(name:"name1")
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();
    api.assert_schema()
        .assert_table("A", |table| table.assert_has_column("name1"));

    let dm2 = r#"
        model A {
            id String @id @default(cuid())
            field String @map(name:"name2")
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();
    api.assert_schema().assert_table("A", |t| {
        t.assert_columns_count(2)
            .assert_has_column("id")
            .assert_has_column("name2")
    });
}

#[test_connector(preview_features("referentialActions"))]
fn reordering_and_altering_models_at_the_same_time_works(api: TestApi) {
    let dm1 = r#"
        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

        model A {
            id Int @id
            name Int @unique
            c C @relation(name: "atoc", fields: [name], references: [name], onUpdate: NoAction)
            cs C[] @relation(name: "ctoa")
        }

        model B {
            id Int @id
            name Int @unique
            c C @relation(name: "btoc", fields: [name], references: [name], onUpdate: NoAction)
        }

        model C {
            id Int @id
            name Int @unique
            a A @relation(name: "ctoa", fields: [name], references: [name], onUpdate: NoAction)
            as A[] @relation(name: "atoc")
            bs B[] @relation(name: "btoc")
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();

    let dm2 = r#"
        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

        model C {
            id Int @id
            a A @relation(name: "ctoa2", fields: [name], references: [name], onUpdate: NoAction)
            name Int @unique
            bs B[] @relation(name: "btoc2")
            as A[] @relation(name: "atoc2")
        }

        model A {
            id Int @id
            name Int @unique
            c C @relation(name: "atoc2", fields: [name], references: [name], onUpdate: NoAction)
            cs C[] @relation(name: "ctoa2")
        }

        model B {
            c C @relation(name: "btoc2", fields: [name], references: [name], onUpdate: NoAction)
            name Int @unique
            id Int @id
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang();
}

#[test_connector]
fn switching_databases_must_work(api: TestApi) {
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

    api.schema_push(dm1).send().assert_green_bang();

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
        .assert_green_bang();
}

#[test_connector(tags(Sqlite))]
fn renaming_a_datasource_works(api: TestApi) {
    let dm1 = r#"
        datasource db1 {
            provider = "sqlite"
            url = "file:///tmp/prisma-test.db"
        }

        model User {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();

    let dm2 = r#"
        datasource db2 {
            provider = "sqlite"
            url = "file:///tmp/prisma-test.db"
        }

        model User {
            id Int @id
        }
    "#;

    api.schema_push(dm2)
        .migration_id(Some("mig02"))
        .send()
        .assert_green_bang()
        .assert_no_steps();
}

#[test_connector]
fn simple_type_aliases_in_migrations_must_work(api: TestApi) {
    let dm1 = r#"
        type CUID = String @id @default(cuid())

        model User {
            id CUID
            age Float
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();
}

#[test_connector]
fn created_at_does_not_get_arbitrarily_migrated(api: TestApi) {
    use quaint::ast::Insert;

    let dm1 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();
    api.assert_schema().assert_table("Fruit", |t| {
        t.assert_column("createdAt", |c| c.assert_default(Some(DefaultValue::now())))
    });

    let insert = Insert::single_into(api.render_table_name("Fruit")).value("name", "banana");
    api.query(insert.into());

    let dm2 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    api.schema_push(dm2).send().assert_green_bang().assert_no_steps();
}

#[test_connector]
fn basic_compound_primary_keys_must_work(api: TestApi) {
    let dm = r#"
        model User {
            firstName String
            lastName String

            @@id([lastName, firstName])
        }
    "#;

    api.schema_push(dm).send().assert_green_bang();

    api.assert_schema().assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["lastName", "firstName"]))
    });
}

#[test_connector]
fn compound_primary_keys_on_mapped_columns_must_work(api: TestApi) {
    let dm = r#"
        model User {
            firstName String @map("first_name")
            lastName String @map("family_name")

            @@id([firstName, lastName])
        }
    "#;

    api.schema_push(dm).send().assert_green_bang();

    api.assert_schema().assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["first_name", "family_name"]))
    });
}
