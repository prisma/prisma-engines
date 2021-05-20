use migration_engine_tests::sync_test_api::*;
use sql_schema_describer::ForeignKeyAction;

#[test_connector]
fn foreign_keys_of_inline_one_to_one_relations_have_a_unique_constraint(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int   @id
            box Box?
        }

        model Box {
            id     Int @id
            cat_id Int
            cat    Cat @relation(fields: [cat_id], references: [id])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("Box", |t| {
        t.assert_indexes_count(1)
            .unwrap()
            .assert_index_on_columns(&["cat_id"], |idx| {
                idx.assert_is_unique().unwrap().assert_name("Box_cat_id_unique")
            })
    });
}

#[test_connector]
fn foreign_keys_are_added_on_existing_tables(api: TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        // There should be no foreign keys yet.
        .assert_table("Account", |table| table.assert_foreign_keys_count(0))
        .unwrap();

    let dm2 = r#"
        model User {
            id    Int @id
            email String @unique
            a     Account[]
        }

        model Account {
            id Int @id
            user_email String
            user User @relation(fields: [user_email], references: [email])
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .unwrap()
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    });
}

#[test_connector]
fn foreign_keys_can_be_added_on_existing_columns(api: TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
            user_email String
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        // There should be no foreign keys yet.
        .assert_table("Account", |table| table.assert_foreign_keys_count(0))
        .unwrap();

    let dm2 = r#"
        model User {
            id    Int @id
            email String @unique
            a     Account[]
        }

        model Account {
            id Int @id
            user_email String
            user User @relation(fields: [user_email], references: [email])
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .unwrap()
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    });
}

#[test_connector]
fn foreign_keys_can_be_dropped_on_existing_columns(api: TestApi) {
    let dm1 = r#"
        model User {
            id Int @id
            email String @unique
            a     Account[]
        }

        model Account {
            id Int @id
            user_email String
            user User @relation(fields: [user_email], references: [email])
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Account", |table| {
            table
                .assert_foreign_keys_count(1)
                .unwrap()
                .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
        })
        .unwrap();

    let dm2 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
            user_email String
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table_bang("Account", |table| table.assert_foreign_keys_count(0));
}

#[test_connector]
fn changing_a_scalar_field_to_a_relation_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b  String
        }

        model B {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema()
        .assert_table("A", |t| {
            t.assert_column("b", |c| c.assert_type_is_string())?
                .assert_foreign_keys_count(0)?
                .assert_indexes_count(0)
        })
        .unwrap();

    let dm2 = r#"
        model A {
            id Int @id
            b Int
            b_rel B @relation(fields: [b], references: [id])
        }

        model B {
            id Int @id
            a  A?
        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send_sync()
        .assert_executable()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_column("b", |col| col.assert_type_is_int())?
            .assert_fk_on_columns(&["b"], |fk| fk.assert_references("B", &["id"]))
    });
}

#[test_connector]
fn changing_a_relation_field_to_a_scalar_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b Int
            b_rel B @relation(fields: [b], references: [id], onDelete: Cascade)
        }

        model B {
            id Int @id
            a  A?
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("A", |table| {
            table
                .assert_column("b", |col| col.assert_type_is_int())
                .unwrap()
                .assert_foreign_keys_count(1)
                .unwrap()
                .assert_fk_on_columns(&["b"], |fk| {
                    fk.assert_references("B", &["id"])?
                        .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
                })
        })
        .unwrap();

    let dm2 = r#"
        model A {
            id Int @id
            b String
        }
        model B {
            id Int @id
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("A", |table| {
            table
                .assert_column("b", |col| col.assert_type_is_string())?
                .assert_foreign_keys_count(0)
        })
        .unwrap();
}

#[test_connector]
fn changing_a_foreign_key_constrained_column_from_nullable_to_required_and_back_works(api: TestApi) {
    let dm = r#"
        model Student {
            id       String @id @default(cuid())
            name     String
            career   Career? @relation(fields: [careerId], references: [id])
            careerId String?
        }
        model Career {
            id       String    @id @default(cuid())
            name     String
            students Student[]
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    let dm2 = r#"
        model Student {
            id       String @id @default(cuid())
            name     String
            career   Career @relation(fields: [careerId], references: [id])
            careerId String
        }
        model Career {
            id       String    @id @default(cuid())
            name     String
            students Student[]
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();
    api.schema_push(dm).send_sync().assert_green_bang();
}

// TODO: Enable SQL Server when cascading rules are in PSL.
#[test_connector(exclude(Mssql))]
fn changing_all_referenced_columns_of_foreign_key_works(api: TestApi) {
    let dm1 = r#"
       model Post {
          id        Int     @default(autoincrement()) @id
          author    User?   @relation(fields: [authorId], references: [id])
          authorId  Int?
        }

        model User {
          id       Int     @default(autoincrement()) @id
          posts    Post[]
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    let dm2 = r#"
        model Post {
          id        Int     @default(autoincrement()) @id
          author    User?   @relation(fields: [authorId], references: [uid])
          authorId  Int?
        }

        model User {
          uid   Int    @id
          posts Post[]
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();
}
