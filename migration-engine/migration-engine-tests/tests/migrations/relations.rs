use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn adding_a_many_to_many_relation_must_result_in_a_prisma_style_relation_table(api: TestApi) {
    let dm1 = r##"
        model A {
            id Int @id
            bs B[]
        }

        model B {
            id String @id
            as A[]
        }
    "##;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("_AToB", |table| {
        table
            .assert_columns_count(2)?
            .assert_column("A", |col| col.assert_type_is_int())?
            .assert_column("B", |col| col.assert_type_is_string())?
            .assert_fk_on_columns(&["A"], |fk| {
                fk.assert_references("A", &["id"])?.assert_cascades_on_delete()
            })?
            .assert_fk_on_columns(&["B"], |fk| {
                fk.assert_references("B", &["id"])?.assert_cascades_on_delete()
            })
    });
}

#[test_connector]
fn adding_a_many_to_many_relation_with_custom_name_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            bs B[] @relation(name: "my_relation")
        }
        model B {
            id Int @id
            as A[] @relation(name: "my_relation")
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("_my_relation", |table| {
        table
            .assert_columns_count(2)?
            .assert_column("A", |col| col.assert_type_is_int())?
            .assert_column("B", |col| col.assert_type_is_int())?
            .assert_foreign_keys_count(2)?
            .assert_fk_on_columns(&["A"], |fk| fk.assert_references("A", &["id"]))?
            .assert_fk_on_columns(&["B"], |fk| fk.assert_references("B", &["id"]))
    });
}

#[test_connector]
fn adding_an_inline_relation_must_result_in_a_foreign_key_in_the_model_table(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            bid Int
            cid Int?
            b  B   @relation(fields: [bid], references: [id])
            c  C?  @relation(fields: [cid], references: [id])
        }

        model B {
            id Int @id
            a  A[]
        }

        model C {
            id Int @id
            a  A[]
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_column("bid", |c| c.assert_type_is_int()?.assert_is_required())?
            .assert_column("cid", |c| c.assert_type_is_int()?.assert_is_nullable())?
            .assert_foreign_keys_count(2)?
            .assert_fk_on_columns(&["bid"], |fk| {
                fk.assert_references("B", &["id"])?.assert_cascades_on_delete()
            })?
            .assert_fk_on_columns(&["cid"], |fk| fk.assert_references("C", &["id"]))
    });
}

#[test_connector]
fn specifying_a_db_name_for_an_inline_relation_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b_id_field Int @map(name: "b_column")
            b B @relation(fields: [b_id_field], references: [id])
        }

        model B {
            id Int @id
            a  A[]
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_column("b_column", |c| c.assert_type_is_int())?
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["b_column"], |fk| fk.assert_references("B", &["id"]))
    });
}

#[test_connector]
fn adding_an_inline_relation_to_a_model_with_an_exotic_id_type(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b_id String
            b B @relation(fields: [b_id], references: [id])
        }

        model B {
            id String @id @default(cuid())
            a  A[]
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_column("b_id", |c| c.assert_type_is_string())?
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["b_id"], |fk| {
                fk.assert_references("B", &["id"])?.assert_cascades_on_delete()
            })
    });
}

#[test_connector]
fn removing_an_inline_relation_must_work(api: TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                b_id Int
                b B @relation(fields: [b_id], references: [id])
            }

            model B {
                id Int @id
                a  A[]
            }
        "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table_bang("A", |table| table.assert_has_column("b_id"));

    let dm2 = r#"
            model A {
                id Int @id
            }

            model B {
                id Int @id
            }
        "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_foreign_keys_count(0)?
            .assert_indexes_count(0)?
            .assert_does_not_have_column("b")
    });
}

#[test_connector]
fn compound_foreign_keys_should_work_in_correct_order(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b Int
            a Int
            d Int
            bb B @relation(fields: [a, b, d], references: [a_id, b_id, d_id])
        }

        model B {
            b_id Int
            a_id Int
            d_id Int
            a    A[]
            @@id([a_id, b_id, d_id])
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["a", "b", "d"], |fk| {
                fk.assert_cascades_on_delete()?
                    .assert_references("B", &["a_id", "b_id", "d_id"])
            })
    });
}

#[test_connector]
fn moving_an_inline_relation_to_the_other_side_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b_id Int
            b B @relation(fields: [b_id], references: [id])
        }

        model B {
            id Int @id
            a  A[]
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_foreign_keys_count(1)?.assert_fk_on_columns(&["b_id"], |fk| {
            fk.assert_cascades_on_delete()?.assert_references("B", &["id"])
        })
    });

    let dm2 = r#"
        model A {
            id Int @id
            b  B[]
        }

        model B {
            id Int @id
            a_id Int
            a A @relation(fields: [a_id], references: [id])
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();
    api.assert_schema()
        .assert_table_bang("B", |table| {
            table
                .assert_foreign_keys_count(1)?
                .assert_fk_on_columns(&["a_id"], |fk| {
                    fk.assert_references("A", &["id"])?.assert_cascades_on_delete()
                })
        })
        .assert_table_bang("A", |table| table.assert_foreign_keys_count(0)?.assert_indexes_count(0));
}

#[test_connector]
fn relations_can_reference_arbitrary_unique_fields(api: TestApi) {
    let dm = r#"
        model User {
            id Int @id
            email String @unique
            a     Account[]
        }

        model Account {
            id Int @id
            uem String
            user User @relation(fields: [uem], references: [email])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("Account", |t| {
        t.assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["uem"], |fk| fk.assert_references("User", &["email"]))
    });
}

#[test_connector]
fn relations_can_reference_arbitrary_unique_fields_with_maps(api: TestApi) {
    let dm = r#"
        model User {
            id Int @id
            email String @unique @map("emergency-mail")
            accounts Account[]

            @@map("users")
        }

        model Account {
            id Int @id
            uem String @map("user-id")
            user User @relation(fields: [uem], references: [email])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["user-id"], |fk| fk.assert_references("users", &["emergency-mail"]))
    });
}

#[test_connector]
fn relations_can_reference_multiple_fields(api: TestApi) {
    let dm = r#"
        model User {
            id Int @id
            email  String
            age    Int
            a      Account[]

            @@unique([email, age])
        }

        model Account {
            id   Int @id
            usermail String
            userage Int
            user User @relation(fields: [usermail, userage], references: [email, age])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["usermail", "userage"], |fk| {
                fk.assert_references("User", &["email", "age"])
            })
    });
}

#[test_connector]
fn a_relation_with_mappings_on_both_sides_can_reference_multiple_fields(api: TestApi) {
    let dm = r#"
        model User {
            id Int @id
            email  String @map("emergency-mail")
            age    Int    @map("birthdays-count")
            a      Account[]

            @@unique([email, age])
            @@map("users")
        }

        model Account {
            id   Int @id
            usermail String @map("emergency-mail-fk-1")
            userage Int @map("age-fk2")

            user User @relation(fields: [usermail, userage], references: [email, age])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["emergency-mail-fk-1", "age-fk2"], |fk| {
                fk.assert_references("users", &["emergency-mail", "birthdays-count"])
            })
    });
}

#[test_connector]
fn relations_with_mappings_on_referenced_side_can_reference_multiple_fields(api: TestApi) {
    let dm = r#"
        model User {
            id Int @id
            email  String @map("emergency-mail")
            age    Int    @map("birthdays-count")
            a      Account[]

            @@unique([email, age])
            @@map("users")
        }

        model Account {
            id   Int @id
            useremail String
            userage Int
            user User @relation(fields: [useremail, userage], references: [email, age])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["useremail", "userage"], |fk| {
                fk.assert_references("users", &["emergency-mail", "birthdays-count"])
            })
    });
}

#[test_connector]
fn relations_with_mappings_on_referencing_side_can_reference_multiple_fields(api: TestApi) {
    let dm = r#"
        model User {
            id Int @id
            email  String
            age    Int
            a      Account[]

            @@unique([email, age])
            @@map("users")
        }

        model Account {
            id   Int @id
            user_email String @map("emergency-mail-fk1")
            user_age Int @map("age-fk2")
            user User @relation(fields: [user_email, user_age], references: [email, age])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["emergency-mail-fk1", "age-fk2"], |fk| {
                fk.assert_references("users", &["email", "age"])
            })
    });
}
