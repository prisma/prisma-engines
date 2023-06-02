use sql_migration_tests::test_api::*;
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
            cat_id Int @unique
            cat    Cat @relation(fields: [cat_id], references: [id])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();
    api.assert_schema().assert_table("Box", |t| {
        t.assert_indexes_count(1)
            .assert_index_on_columns(&["cat_id"], |idx| idx.assert_is_unique().assert_name("Box_cat_id_key"))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        // There should be no foreign keys yet.
        .assert_table("Account", |table| table.assert_foreign_keys_count(0));

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

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        // There should be no foreign keys yet.
        .assert_table("Account", |table| table.assert_foreign_keys_count(0));

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

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    });

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

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("Account", |table| table.assert_foreign_keys_count(0));
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_column("b", |c| c.assert_type_is_string())
            .assert_foreign_keys_count(0)
            .assert_indexes_count(0)
    });

    let dm2 = r#"
        model A {
            id Int  @id
            b Int   @unique
            b_rel B @relation(fields: [b], references: [id])
        }

        model B {
            id Int @id
            a  A?
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("b", |col| col.assert_type_is_int())
            .assert_fk_on_columns(&["b"], |fk| fk.assert_references("B", &["id"]))
    });
}

#[test_connector(exclude(Vitess))]
fn changing_a_relation_field_to_a_scalar_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int  @id
            b Int   @unique
            b_rel B @relation(fields: [b], references: [id])
        }

        model B {
            id Int @id
            a  A?
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("b", |col| col.assert_type_is_int())
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["b"], |fk| {
                let on_delete = if api.is_mssql() {
                    ForeignKeyAction::NoAction
                } else {
                    ForeignKeyAction::Restrict
                };

                fk.assert_references("B", &["id"])
                    .assert_referential_action_on_delete(on_delete)
            })
    });

    let dm2 = r#"
        model A {
            id Int @id
            b String
        }
        model B {
            id Int @id
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("b", |col| col.assert_type_is_string())
            .assert_foreign_keys_count(0)
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm).send().assert_green();

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

    api.schema_push_w_datasource(dm2).send().assert_green();
    api.schema_push_w_datasource(dm).send().assert_green();
}

#[test_connector(exclude(CockroachDb))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

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

    api.schema_push_w_datasource(dm2).send().assert_green();
}
