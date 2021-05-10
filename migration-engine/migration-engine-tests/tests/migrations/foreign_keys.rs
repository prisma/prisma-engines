use migration_engine_tests::multi_engine_test_api::*;

#[test_connector]
fn foreign_keys_of_inline_one_to_one_relations_have_a_unique_constraint(api: TestApi) {
    let engine = api.new_engine();

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

    engine.schema_push(dm).send_sync().unwrap().assert_green().unwrap();
    engine
        .assert_schema()
        .assert_table("Box", |t| {
            t.assert_indexes_count(1)
                .unwrap()
                .assert_index_on_columns(&["cat_id"], |idx| {
                    idx.assert_is_unique().unwrap().assert_name("Box_cat_id_unique")
                })
        })
        .unwrap();
}

#[test_connector]
fn foreign_keys_are_added_on_existing_tables(api: TestApi) -> TestResult {
    let api = api.new_engine();

    let dm1 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

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

    api.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();

    api.assert_schema()
        .assert_table("Account", |table| {
            table
                .assert_foreign_keys_count(1)
                .unwrap()
                .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
        })
        .unwrap();
}

#[test_connector]
fn foreign_keys_can_be_added_on_existing_columns(api: TestApi) -> TestResult {
    let api = api.new_engine();
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

    api.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

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

    api.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();

    api.assert_schema()
        .assert_table("Account", |table| {
            table
                .assert_foreign_keys_count(1)
                .unwrap()
                .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
        })
        .unwrap();
}

#[test_connector]
fn foreign_keys_can_be_dropped_on_existing_columns(api: TestApi) {
    let api = api.new_engine();
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

    api.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

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

    api.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();

    api.assert_schema()
        .assert_table("Account", |table| table.assert_foreign_keys_count(0))
        .unwrap();
}

#[test_connector]
fn changing_a_scalar_field_to_a_relation_field_must_work(api: TestApi) {
    let api = api.new_engine();
    let dm1 = r#"
        model A {
            id Int @id
            b  String
        }

        model B {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();
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
        .unwrap()
        .assert_executable()
        .unwrap()
        .assert_has_executed_steps()
        .unwrap();

    api.assert_schema()
        .assert_table("A", |table| {
            table
                .assert_column("b", |col| col.assert_type_is_int())?
                .assert_fk_on_columns(&["b"], |fk| fk.assert_references("B", &["id"]))
        })
        .unwrap();
}

#[test_connector]
fn changing_a_relation_field_to_a_scalar_field_must_work(api: TestApi) {
    let api = api.new_engine();
    let dm1 = r#"
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

    api.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    api.assert_schema()
        .assert_table("A", |table| {
            table
                .assert_column("b", |col| col.assert_type_is_int())
                .unwrap()
                .assert_foreign_keys_count(1)
                .unwrap()
                .assert_fk_on_columns(&["b"], |fk| {
                    fk.assert_references("B", &["id"])?.assert_cascades_on_delete()
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

    api.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();

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
    let api = api.new_engine();

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

    api.schema_push(dm).send_sync().unwrap().assert_green().unwrap();

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

    api.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();
    api.schema_push(dm).send_sync().unwrap().assert_green().unwrap();
}
