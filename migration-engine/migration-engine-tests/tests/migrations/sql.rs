use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn can_handle_reserved_sql_keywords_for_model_name(api: TestApi) {
    let dm1 = r#"
        model Group {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema()
        .assert_table_bang("Group", |t| t.assert_column("field", |c| c.assert_type_is_string()));

    let dm2 = r#"
        model Group {
            id String @id @default(cuid())
            field Int
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();
    api.assert_schema()
        .assert_table_bang("Group", |t| t.assert_column("field", |c| c.assert_type_is_int()));
}

#[test_connector]
fn can_handle_reserved_sql_keywords_for_field_name(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            Group String
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema()
        .assert_table_bang("Test", |t| t.assert_column("Group", |c| c.assert_type_is_string()));

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            Group Int
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();
    api.assert_schema()
        .assert_table_bang("Test", |t| t.assert_column("Group", |c| c.assert_type_is_int()));
}

#[test_connector]
fn creating_tables_without_primary_key_must_work(api: TestApi) {
    let dm = r#"
        model Pair {
            index Int
            name String
            weight Float

            @@unique([index, name])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Pair", |table| {
        table
            .assert_has_no_pk()?
            .assert_index_on_columns(&["index", "name"], |idx| idx.assert_is_unique())
    });
}

#[test_connector]
fn relations_to_models_without_a_primary_key_work(api: TestApi) {
    let dm = r#"
        model Pair {
            index Int
            name String
            weight Float
            pm     PairMetadata[]

            @@unique([index, name])
        }

        model PairMetadata {
            id String @id
            pairidx Int
            pairname String
            pair Pair @relation(fields: [pairidx, pairname], references: [index, name])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table_bang("Pair", |table| table.assert_has_no_pk())
        .assert_table_bang("PairMetadata", |table| {
            table
                .assert_pk(|pk| pk.assert_columns(&["id"]))?
                .assert_fk_on_columns(&["pairidx", "pairname"], |fk| {
                    fk.assert_references("Pair", &["index", "name"])
                })
        });
}

#[test_connector]
fn relations_to_models_with_no_pk_and_a_single_unique_required_field_work(api: TestApi) {
    let dm = r#"
        model Pair {
            index Int
            name String
            weight Float @unique
            pm     PairMetadata[]
        }

        model PairMetadata {
            id String @id
            pweight Float
            pair Pair @relation(fields: [pweight], references: [weight])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table_bang("Pair", |table| table.assert_has_no_pk())
        .assert_table_bang("PairMetadata", |table| {
            table
                .assert_pk(|pk| pk.assert_columns(&["id"]))?
                .assert_fk_on_columns(&["pweight"], |fk| fk.assert_references("Pair", &["weight"]))
        });
}

#[test_connector(capabilities(Enums))]
fn enum_value_with_database_names_must_work(api: TestApi) {
    let dm = r##"
        model Cat {
            id String @id
            mood CatMood
        }

        enum CatMood {
            ANGRY
            HUNGRY @map("hongry")
        }
    "##;

    api.schema_push(dm)
        .migration_id(Some("initial"))
        .send_sync()
        .assert_green_bang();

    if api.is_mysql() {
        api.assert_schema()
            .assert_enum(&api.normalize_identifier("Cat_mood"), |enm| {
                enm.assert_values(&["ANGRY", "hongry"])
            })
            .unwrap();
    } else {
        api.assert_schema()
            .assert_enum("CatMood", |enm| enm.assert_values(&["ANGRY", "hongry"]))
            .unwrap();
    }

    let dm = r##"
        model Cat {
            id String @id
            mood CatMood
        }

        enum CatMood {
            ANGRY
            HUNGRY @map("hongery")
        }
    "##;

    if api.is_mysql() {
        api.schema_push(dm).force(true).send_sync().assert_warnings(&["The values [hongry] on the enum `Cat_mood` will be removed. If these variants are still used in the database, this will fail.".into()]);

        api.assert_schema()
            .assert_enum(&api.normalize_identifier("Cat_mood"), |enm| {
                enm.assert_values(&["ANGRY", "hongery"])
            })
            .unwrap();
    } else {
        api.schema_push(dm).force(true).send_sync().assert_warnings(&["The values [hongry] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail.".into()]);
        api.assert_schema()
            .assert_enum("CatMood", |enm| enm.assert_values(&["ANGRY", "hongery"]))
            .unwrap();
    }
}

#[test_connector(capabilities(Enums))]
fn enum_defaults_must_work(api: TestApi) {
    let dm = r##"
        model Cat {
            id String @id
            mood CatMood @default(HUNGRY)
            previousMood CatMood @default(ANGRY)
        }

        enum CatMood {
            ANGRY
            HUNGRY @map("hongry")
        }
    "##;

    api.schema_push(dm)
        .migration_id(Some("initial"))
        .send_sync()
        .assert_green_bang();

    let insert = quaint::ast::Insert::single_into(api.render_table_name("Cat")).value("id", "the-id");
    api.query(insert.into());

    let row = api
        .query(
            quaint::ast::Select::from_table(api.render_table_name("Cat"))
                .column("id")
                .column("mood")
                .column("previousMood")
                .into(),
        )
        .into_single()
        .unwrap();

    assert_eq!(row.get("id").unwrap().to_string().unwrap(), "the-id");
    assert_eq!(
        match row.get("mood").unwrap() {
            quaint::Value::Enum(Some(enm)) => enm.as_ref(),
            quaint::Value::Text(Some(enm)) => enm.as_ref(),
            _ => panic!("mood is not an enum value"),
        },
        "hongry"
    );
    assert_eq!(
        match row.get("previousMood").unwrap() {
            quaint::Value::Enum(Some(enm)) => enm.as_ref(),
            quaint::Value::Text(Some(enm)) => enm.as_ref(),
            _ => panic!("previousMood is not an enum value"),
        },
        "ANGRY"
    );
}

#[test_connector]
fn id_as_part_of_relation_must_work(api: TestApi) {
    let dm = r##"
        model Cat {
            nemesis_id String @id
            nemesis Dog @relation(fields: [nemesis_id], references: [id])
        }

        model Dog {
            id    String @id
            cats  Cat[]
        }
    "##;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["nemesis_id"]))?
            .assert_fk_on_columns(&["nemesis_id"], |fk| fk.assert_references("Dog", &["id"]))
    });
}

#[test_connector]
fn multi_field_id_as_part_of_relation_must_work(api: TestApi) {
    let dm = r##"
        model Cat {
            nemesis_name String
            nemesis_weight Int

            nemesis Dog @relation(fields: [nemesis_name, nemesis_weight], references: [name, weight])

            @@id([nemesis_name, nemesis_weight])
        }

        model Dog {
            name String
            weight Int
            cats    Cat[]

            @@id([name, weight])
        }
    "##;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["nemesis_name", "nemesis_weight"]))?
            .assert_fk_on_columns(&["nemesis_name", "nemesis_weight"], |fk| {
                fk.assert_references("Dog", &["name", "weight"])
            })
    });
}

#[test_connector]
fn remapped_multi_field_id_as_part_of_relation_must_work(api: TestApi) {
    let dm = r##"
        model Cat {
            nemesis_name String @map("dogname")
            nemesis_weight Int @map("dogweight")
            nemesis Dog @relation(fields: [nemesis_name, nemesis_weight], references: [name, weight])

            @@id([nemesis_name, nemesis_weight])
        }

        model Dog {
            name String
            weight Int
            cats   Cat[]

            @@id([name, weight])
        }
    "##;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["dogname", "dogweight"]))?
            .assert_fk_on_columns(&["dogname", "dogweight"], |fk| {
                fk.assert_references("Dog", &["name", "weight"])
            })
    });
}

#[test_connector]
fn unique_constraints_on_composite_relation_fields(api: TestApi) {
    let dm = r##"
        model Parent {
            id    Int    @id
            chiid Int
            chic  String
            child Child  @relation(fields: [chiid, chic], references: [id, c])
            p     String

            @@unique([chiid, chic])
        }

        model Child {
            id        Int    @id
            c         String
            p         Parent[]

            @@unique([id, c])
        }
    "##;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Parent", |table| {
        table.assert_index_on_columns(&["chiid", "chic"], |idx| idx.assert_is_unique())
    });
}

#[test_connector]
fn indexes_on_composite_relation_fields(api: TestApi) {
    let dm = r##"
        model User {
          id                  Int       @id
          firstName           String
          lastName            String
          s                   SpamList[]

          @@unique([firstName, lastName])
        }

        model SpamList {
          id   Int  @id
          ufn String
          uln String
          user User @relation(fields: [ufn, uln], references: [firstName, lastName])

          @@index([ufn, uln])
        }
    "##;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("SpamList", |table| {
        table.assert_index_on_columns(&["ufn", "uln"], |idx| idx.assert_is_not_unique())
    });
}

#[test_connector(exclude(Mssql))]
fn dropping_mutually_referencing_tables_works(api: TestApi) {
    let dm1 = r#"
    model A {
        id Int @id
        b_id Int
        ab B @relation("AtoB", fields: [b_id], references: [id])
        c_id Int
        ac C @relation("AtoC", fields: [c_id], references: [id])
        b  B[] @relation("BtoA")
        c  C[] @relation("CtoA")
    }

    model B {
        id Int @id
        a_id Int
        ba A @relation("BtoA", fields: [a_id], references: [id])
        c_id Int
        bc C @relation("BtoC", fields: [c_id], references: [id])
        a  A[] @relation("AtoB")
        c  C[] @relation("CtoB")
    }

    model C {
        id Int @id
        a_id Int
        ca A @relation("CtoA", fields: [a_id], references: [id])
        b_id Int
        cb B @relation("CtoB", fields: [b_id], references: [id])
        b  B[] @relation("BtoC")
        a  A[] @relation("AtoC")
    }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_tables_count(3).unwrap();

    api.schema_push("").send_sync().assert_green_bang();
    api.assert_schema().assert_tables_count(0).unwrap();
}
