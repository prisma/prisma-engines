mod cockroachdb;
mod vitess;

use psl::parser_database::ReferentialAction;
use sql_migration_tests::test_api::*;
use sql_schema_describer::{ColumnTypeFamily, ForeignKeyAction};

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("_AToB", |table| {
        table
            .assert_columns_count(2)
            .assert_column("A", |col| col.assert_type_is_int())
            .assert_column("B", |col| col.assert_type_is_string())
            .assert_fk_on_columns(&["A"], |fk| {
                fk.assert_references("A", &["id"])
                    .assert_referential_action_on_update(ForeignKeyAction::Cascade)
                    .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
            })
            .assert_fk_on_columns(&["B"], |fk| {
                fk.assert_references("B", &["id"])
                    .assert_referential_action_on_update(ForeignKeyAction::Cascade)
                    .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
            })
    });

    // Check that the migration is idempotent
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("_my_relation", |table| {
        table
            .assert_columns_count(2)
            .assert_column("A", |col| col.assert_type_is_int())
            .assert_column("B", |col| col.assert_type_is_int())
            .assert_foreign_keys_count(2)
            .assert_fk_on_columns(&["A"], |fk| fk.assert_references("A", &["id"]))
            .assert_fk_on_columns(&["B"], |fk| fk.assert_references("B", &["id"]))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_column("bid", |c| c.assert_type_is_int().assert_is_required())
            .assert_column("cid", |c| c.assert_type_is_int().assert_is_nullable())
            .assert_foreign_keys_count(2)
            .assert_fk_on_columns(&["bid"], |fk| {
                fk.assert_references("B", &["id"])
                    .assert_referential_action_on_update(ForeignKeyAction::Cascade)
                    .assert_referential_action_on_delete(if api.is_mssql() {
                        ForeignKeyAction::NoAction
                    } else {
                        ForeignKeyAction::Restrict
                    })
            })
            .assert_fk_on_columns(&["cid"], |fk| fk.assert_references("C", &["id"]))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_column("b_column", |c| c.assert_type_is_int())
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["b_column"], |fk| fk.assert_references("B", &["id"]))
    });
}

#[test_connector(exclude(Vitess))]
fn changing_the_type_of_a_field_referenced_by_a_fk_must_work(api: TestApi) {
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::Int))
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["uniq"]))
    });

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

    api.schema_push_w_datasource(dm2).send().assert_green();

    // TODO(2022-02-25): it appears that the migration above _does_ re-add the foreign key on
    // cockroachdb, but (the logs show an ALTER TABLE ADD CONSTRAINT FOREIGN KEY) but it doesn't
    // appear in the information schema. I haven't found an issue for this on the cockroach issue
    // tracker. We should do a minimal reproduction and open an issue.
    // Apart from this, it looks like this test would pass on cockroach.
    if api.is_cockroach() {
        return;
    }

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::String))
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["uniq"]))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_column("b_id", |c| c.assert_type_is_string())
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["b_id"], |fk| {
                fk.assert_references("B", &["id"])
                    .assert_referential_action_on_update(ForeignKeyAction::Cascade)
                    .assert_referential_action_on_delete(if api.is_mssql() {
                        ForeignKeyAction::NoAction
                    } else {
                        ForeignKeyAction::Restrict
                    })
            })
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_has_column("b_id"));

    let dm2 = r#"
            model A {
                id Int @id
            }

            model B {
                id Int @id
            }
        "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_foreign_keys_count(0)
            .assert_indexes_count(0)
            .assert_does_not_have_column("b")
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |t| {
        t.assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["a", "b", "d"], |fk| {
                fk.assert_referential_action_on_delete(if api.is_mssql() {
                    ForeignKeyAction::NoAction
                } else {
                    ForeignKeyAction::Restrict
                })
                .assert_referential_action_on_update(ForeignKeyAction::Cascade)
                .assert_references("B", &["a_id", "b_id", "d_id"])
            })
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_foreign_keys_count(1).assert_fk_on_columns(&["b_id"], |fk| {
            fk.assert_referential_action_on_delete(if api.is_mssql() {
                ForeignKeyAction::NoAction
            } else {
                ForeignKeyAction::Restrict
            })
            .assert_referential_action_on_update(ForeignKeyAction::Cascade)
            .assert_references("B", &["id"])
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

    api.schema_push_w_datasource(dm2).send().assert_green();
    api.assert_schema()
        .assert_table("B", |table| {
            table
                .assert_foreign_keys_count(1)
                .assert_fk_on_columns(&["a_id"], |fk| {
                    fk.assert_references("A", &["id"])
                        .assert_referential_action_on_delete(if api.is_mssql() {
                            ForeignKeyAction::NoAction
                        } else {
                            ForeignKeyAction::Restrict
                        })
                        .assert_referential_action_on_update(ForeignKeyAction::Cascade)
                })
        })
        .assert_table("A", |table| table.assert_foreign_keys_count(0).assert_indexes_count(0));
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm).send().assert_green();
    api.assert_schema().assert_table("Account", |t| {
        t.assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["uem"], |fk| fk.assert_references("User", &["email"]))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["user-id"], |fk| fk.assert_references("users", &["emergency-mail"]))
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["usermail", "userage"], |fk| {
                fk.assert_references("User", &["email", "age"])
            })
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["emergency-mail-fk-1", "age-fk2"], |fk| {
                fk.assert_references("users", &["emergency-mail", "birthdays-count"])
            })
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["useremail", "userage"], |fk| {
                fk.assert_references("users", &["emergency-mail", "birthdays-count"])
            })
    });
}

#[test_connector(exclude(Vitess))]
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["emergency-mail-fk1", "age-fk2"], |fk| {
                fk.assert_references("users", &["email", "age"])
            })
    });
}

#[test_connector(exclude(Vitess, CockroachDb))]
fn on_delete_referential_actions_should_work(api: TestApi) {
    let actions = &[
        (ReferentialAction::SetNull, ForeignKeyAction::SetNull),
        (ReferentialAction::Cascade, ForeignKeyAction::Cascade),
        (ReferentialAction::NoAction, ForeignKeyAction::NoAction),
    ];

    for (ra, fka) in actions {
        let dm = format!(
            r#"
            model A {{
                id Int @id @default(autoincrement())
                b      B[]
            }}

            model B {{
                id   Int @id
                aId  Int?
                a    A?    @relation(fields: [aId], references: [id], onDelete: {ra})
            }}
        "#
        );

        api.schema_push_w_datasource(&dm).send().assert_green();

        api.assert_schema().assert_table("B", |table| {
            table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
                fk.assert_references("A", &["id"])
                    .assert_referential_action_on_delete(*fka)
            })
        });

        api.schema_push_w_datasource("").send().assert_green();
    }
}

// 5.6 and 5.7 doesn't let you `SET DEFAULT` without setting the default value
// (even if nullable). MySQL 8.0+ & MariaDB 10.0 allow you to create a table with
// `SET DEFAULT`, but will silently use `NO ACTION` / `RESTRICT` instead.
#[test_connector(exclude(Mysql56, Mysql57, Mariadb, Mssql, Vitess, CockroachDb))]
fn on_delete_set_default_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int
            a    A    @relation(fields: [aId], references: [id], onDelete: SetDefault)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_delete(ForeignKeyAction::SetDefault)
        })
    });
}

#[test_connector(exclude(Mssql, Vitess))]
fn on_delete_restrict_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int
            a    A    @relation(fields: [aId], references: [id], onDelete: Restrict)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_delete(ForeignKeyAction::Restrict)
        })
    });
}

#[test_connector(exclude(Vitess))]
fn on_update_referential_actions_should_work(api: TestApi) {
    let actions = &[
        (ReferentialAction::NoAction, ForeignKeyAction::NoAction),
        (ReferentialAction::SetNull, ForeignKeyAction::SetNull),
        (ReferentialAction::Cascade, ForeignKeyAction::Cascade),
    ];

    for (ra, fka) in actions {
        let dm = format!(
            r#"
            model A {{
                id BigInt @id @default(autoincrement())
                b      B[]
            }}

            model B {{
                id   BigInt @id
                aId  BigInt?
                a    A?    @relation(fields: [aId], references: [id], onUpdate: {ra})
            }}
        "#
        );

        api.schema_push_w_datasource(&dm).send().assert_green();

        api.assert_schema().assert_table("B", |table| {
            table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
                fk.assert_references("A", &["id"])
                    .assert_referential_action_on_update(*fka)
            })
        });
    }
}

// 5.6 and 5.7 doesn't let you `SET DEFAULT` without setting the default value
// (even if nullable). MySQL 8.0+ & MariaDB 10.0 allow you to create a table with
// `SET DEFAULT`, but will silently use `NO ACTION` / `RESTRICT` instead.
#[test_connector(exclude(Mysql56, Mysql57, Mariadb, Mssql, Vitess, CockroachDb))]
fn on_update_set_default_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int
            a    A    @relation(fields: [aId], references: [id], onUpdate: SetDefault)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_update(ForeignKeyAction::SetDefault)
        })
    });
}

#[test_connector(exclude(Mssql, Vitess))]
fn on_update_restrict_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int
            a    A    @relation(fields: [aId], references: [id], onUpdate: Restrict)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_update(ForeignKeyAction::Restrict)
        })
    });
}

#[test_connector(exclude(Mssql, Vitess))]
fn on_delete_required_default_action(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int
            a    A    @relation(fields: [aId], references: [id])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_delete(ForeignKeyAction::Restrict)
        })
    });
}

#[test_connector(tags(Mssql))]
fn on_delete_required_default_action_with_no_restrict(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int
            a    A    @relation(fields: [aId], references: [id])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_delete(ForeignKeyAction::NoAction)
        })
    });
}

#[test_connector(exclude(Vitess))]
fn on_delete_optional_default_action(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int?
            a    A?    @relation(fields: [aId], references: [id])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_delete(ForeignKeyAction::SetNull)
        })
    });
}

#[test_connector(exclude(Vitess))]
fn on_delete_compound_optional_optional_default_action(api: TestApi) {
    let dm = r#"
        model A {
            id  Int @id
            id2 Int
            b      B[]
            @@unique([id, id2])
        }

        model B {
            id    Int @id
            aId1  Int?
            aId2  Int?
            a     A?    @relation(fields: [aId1, aId2], references: [id, id2])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["aId1", "aId2"], |fk| {
                fk.assert_references("A", &["id", "id2"])
                    .assert_referential_action_on_delete(ForeignKeyAction::SetNull)
            })
    });
}

#[test_connector(exclude(Mssql, Vitess))]
fn on_delete_compound_required_optional_default_action_with_restrict(api: TestApi) {
    let dm = r#"
        model A {
            id  Int @id
            id2 Int
            b      B[]
            @@unique([id, id2])
        }

        model B {
            id    Int @id
            aId1  Int?
            aId2  Int
            a     A?    @relation(fields: [aId1, aId2], references: [id, id2])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["aId1", "aId2"], |fk| {
                fk.assert_references("A", &["id", "id2"])
                    .assert_referential_action_on_delete(ForeignKeyAction::Restrict)
            })
    });
}

#[test_connector(tags(Mssql))]
fn on_delete_compound_required_optional_default_action_without_restrict(api: TestApi) {
    let dm = r#"
        model A {
            id  Int @id
            id2 Int
            b      B[]
            @@unique([id, id2])
        }

        model B {
            id    Int @id
            aId1  Int?
            aId2  Int
            a     A?    @relation(fields: [aId1, aId2], references: [id, id2])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["aId1", "aId2"], |fk| {
                fk.assert_references("A", &["id", "id2"])
                    .assert_referential_action_on_delete(ForeignKeyAction::NoAction)
            })
    });
}

#[test_connector(exclude(Vitess))]
fn on_update_optional_default_action(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int?
            a    A?    @relation(fields: [aId], references: [id])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_update(ForeignKeyAction::Cascade)
        })
    });
}

#[test_connector(exclude(Vitess))]
fn on_update_required_default_action(api: TestApi) {
    let dm = r#"
        model A {
            id Int @id
            b      B[]
        }

        model B {
            id   Int @id
            aId  Int
            a    A    @relation(fields: [aId], references: [id])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("B", |table| {
        table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
            fk.assert_references("A", &["id"])
                .assert_referential_action_on_update(ForeignKeyAction::Cascade)
        })
    });
}

#[test_connector(exclude(Vitess, CockroachDb))]
fn adding_mutual_references_on_existing_tables_works(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
        }

        model B {
            id Int @id
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model A {
            id Int
            name String @unique
            b_email String
            brel B @relation("AtoB", fields: [b_email], references: [email], onDelete: NoAction, onUpdate: NoAction)
            b    B[] @relation("BtoA")
        }

        model B {
            id Int
            email String @unique
            a_name String
            arel A @relation("BtoA", fields: [a_name], references: [name], onDelete: NoAction, onUpdate: NoAction)
            a    A[] @relation("AtoB")
        }
    "#;

    let res = api.schema_push_w_datasource(dm2).force(true).send();

    if api.is_sqlite() {
        res.assert_green();
    } else {
        res.assert_warnings(&["A unique constraint covering the columns `[name]` on the table `A` will be added. If there are existing duplicate values, this will fail.".into(), "A unique constraint covering the columns `[email]` on the table `B` will be added. If there are existing duplicate values, this will fail.".into()]);
    };
}

#[test_connector]
fn migrations_with_many_to_many_related_models_must_not_recreate_indexes(api: TestApi) {
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

    api.schema_push_w_datasource(dm_1).send().assert_green();
    api.assert_schema().assert_table("_ProfileToSkill", |t| {
        if api.is_postgres() && !api.is_cockroach() {
            t.assert_pk(|pk| pk.assert_columns(&["A", "B"]))
        } else {
            t.assert_index_on_columns(&["A", "B"], |idx| idx.assert_is_unique())
        }
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

    api.schema_push_w_datasource(dm_2).send().assert_green();
    api.assert_schema().assert_table("_ProfileToSkill", |table| {
        if api.is_postgres() && !api.is_cockroach() {
            table.assert_pk(|pk| {
                pk.assert_columns(&["A", "B"])
                    .assert_constraint_name("_ProfileToSkill_AB_pkey")
            })
        } else {
            table.assert_index_on_columns(&["A", "B"], |idx| {
                idx.assert_is_unique().assert_name("_ProfileToSkill_AB_unique")
            })
        }
    });

    // Check that the migration is idempotent
    api.schema_push_w_datasource(dm_2)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector]
fn removing_a_relation_field_must_work(api: TestApi) {
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

    api.schema_push_w_datasource(dm_1).send().assert_green();

    api.assert_schema()
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

    api.schema_push_w_datasource(dm_2).send().assert_green();

    api.assert_schema()
        .assert_table("User", |table| table.assert_does_not_have_column("address_name"));
}

#[test_connector(exclude(Vitess))]
fn references_to_models_with_compound_primary_keys_must_work(api: TestApi) {
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Pet", |table| {
        table
            .assert_has_column("id")
            .assert_has_column("human_firstName")
            .assert_has_column("human_lastName")
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
                fk.assert_references("User", &["firstName", "lastName"])
            })
    });
}

#[test_connector(exclude(Vitess))]
fn join_tables_between_models_with_compound_primary_keys_must_work(api: TestApi) {
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("HumanToCat", |table| {
        table
            .assert_has_column("human_firstName")
            .assert_has_column("human_lastName")
            .assert_has_column("cat_id")
            .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
                fk.assert_references("Human", &["firstName", "lastName"])
                    .assert_referential_action_on_delete(if api.is_mssql() {
                        ForeignKeyAction::NoAction
                    } else {
                        ForeignKeyAction::Restrict
                    })
            })
            .assert_fk_on_columns(&["cat_id"], |fk| {
                fk.assert_references("Cat", &["id"])
                    .assert_referential_action_on_delete(if api.is_mssql() {
                        ForeignKeyAction::NoAction
                    } else {
                        ForeignKeyAction::Restrict
                    })
            })
            .assert_indexes_count(2)
            .assert_index_on_columns(&["cat_id", "human_firstName", "human_lastName"], |idx| {
                idx.assert_is_unique()
            })
            .assert_index_on_columns(&["human_firstName", "human_lastName"], |idx| idx.assert_is_not_unique())
    });
}

#[test_connector(exclude(Vitess))]
fn join_tables_between_models_with_mapped_compound_primary_keys_must_work(api: TestApi) {
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("HumanToCat", |table| {
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
}
