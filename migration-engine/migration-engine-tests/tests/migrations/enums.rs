use migration_engine_tests::sync_test_api::*;
use sql_schema_describer::ColumnTypeFamily;

const BASIC_ENUM_DM: &str = r#"
model Cat {
    id Int @id
    mood CatMood
}

enum CatMood {
    HAPPY
    HUNGRY
}
"#;

#[test_connector(capabilities(Enums))]
fn adding_an_enum_field_must_work(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            enum MyEnum
        }

        enum MyEnum {
            A
            B
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Test", |table| {
            table.assert_columns_count(2)?.assert_column("enum", |c| {
                if api.is_postgres() {
                    c.assert_is_required()?
                        .assert_type_family(ColumnTypeFamily::Enum("MyEnum".to_owned()))
                } else if api.is_mysql() {
                    c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Enum(
                        api.normalize_identifier("Test_enum").into_owned(),
                    ))
                } else {
                    c.assert_is_required()?.assert_type_is_string()
                }
            })
        })
        .unwrap();

    // Check that the migration is idempotent.
    api.schema_push(dm).send_sync().assert_no_steps();
}

#[test_connector(capabilities(Enums))]
fn adding_an_enum_field_must_work_with_native_types_off(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            enum MyEnum
        }

        enum MyEnum {
            A
            B
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Test", |table| {
            table.assert_columns_count(2)?.assert_column("enum", |c| {
                if api.is_postgres() {
                    c.assert_is_required()?
                        .assert_type_family(ColumnTypeFamily::Enum("MyEnum".to_owned()))
                } else if api.is_mysql() {
                    c.assert_is_required()?
                        .assert_type_family(ColumnTypeFamily::Enum(api.normalize_identifier("Test_enum").into()))
                } else {
                    c.assert_is_required()?.assert_type_is_string()
                }
            })
        })
        .unwrap();

    // Check that the migration is idempotent.
    api.schema_push(dm).send_sync().assert_no_steps();
}

#[test_connector(capabilities(Enums))]
fn an_enum_can_be_turned_into_a_model(api: TestApi) {
    api.schema_push(BASIC_ENUM_DM).send_sync().assert_green_bang();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    #[allow(clippy::redundant_closure)]
    api.assert_schema().assert_enum(enum_name, |enm| Ok(enm)).unwrap();

    let dm2 = r#"
        model Cat {
            id Int @id
            moodId Int
            mood CatMood @relation(fields: [moodId], references: [id])
        }

        model CatMood {
            id Int @id
            description String
            biteRisk Int
            c        Cat[]
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_columns_count(2)?.assert_column("moodId", Ok)
        })
        .unwrap()
        .assert_table("CatMood", |table| table.assert_column_count(3))
        .unwrap()
        .assert_has_no_enum("CatMood")
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn variants_can_be_added_to_an_existing_enum(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    api.assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]))
        .unwrap();

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
            JOYJOY
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY", "HAPPY", "JOYJOY"]))
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn variants_can_be_removed_from_an_existing_enum(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    api.assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HAPPY", "HUNGRY"]))
        .unwrap();

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    let warning = if api.is_mysql() {
        "The values [HAPPY] on the enum `Cat_mood` will be removed. If these variants are still used in the database, this will fail."
    } else {
        "The values [HAPPY] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail."
    };

    api.schema_push(dm2)
        .force(true)
        .send_sync()
        .assert_warnings(&[warning.into()])
        .assert_executable();

    api.assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]))
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn models_with_enum_values_can_be_dropped(api: TestApi) {
    api.schema_push(BASIC_ENUM_DM).send_sync().assert_green_bang();

    api.assert_schema().assert_tables_count(1).unwrap();

    api.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let warn = if api.lower_cases_table_names() {
        "You are about to drop the `cat` table, which is not empty (1 rows)."
    } else {
        "You are about to drop the `Cat` table, which is not empty (1 rows)."
    };

    api.schema_push("")
        .force(true)
        .send_sync()
        .assert_executable()
        .assert_warnings(&[warn.into()]);

    api.assert_schema().assert_tables_count(0).unwrap();
}

#[test_connector(capabilities(Enums))]
fn enum_field_to_string_field_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_enum())
        })
        .unwrap();

    api.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let dm2 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    api.schema_push(dm2).force(true).send_sync().assert_executable();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        })
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn string_field_to_enum_field_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        })
        .unwrap();

    api.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    let warn = if api.is_postgres() {
        "The `mood` column on the `Cat` table would be dropped and recreated. This will lead to data loss."
    } else if api.lower_cases_table_names() {
        "You are about to alter the column `mood` on the `cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Enum(\"Cat_mood\")`."
    } else {
        "You are about to alter the column `mood` on the `Cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Enum(\"Cat_mood\")`."
    };

    api.schema_push(dm2)
        .force(true)
        .send_sync()
        .assert_executable()
        .assert_warnings(&[warn.into()]);

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_enum())
        })
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn enums_used_in_default_can_be_changed(api: TestApi) {
    let dm1 = r#"
        model Panther {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Tiger {
            id Int @id
            mood CatMood @default(HAPPY)
        }

         model Leopard {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Lion {
            id Int @id
            mood CatMood
        }

        model GoodDog {
            id Int @id
            mood DogMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }

        enum DogMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_tables_count(5).unwrap();

    let dm2 = r#"
        model Panther {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Tiger {
            id Int @id
            mood CatMood @default(HAPPY)
        }

         model Leopard {
            id Int @id
            mood CatMood
        }

        model Lion {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model GoodDog {
            id Int @id
            mood DogMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            ANGRY
        }

        enum DogMood {
            HAPPY
            HUNGRY
            SLEEPY
        }
    "#;

    if api.is_postgres() {
        api.schema_push(dm2)
            .force(true)
            .send_sync()
            .assert_executable()
            .assert_warnings(&["The values [HUNGRY] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail.".into()]
            );
    } else {
        api.schema_push(dm2)
            .force(true)
            .send_sync()
            .assert_executable()
            .assert_warnings(& ["The values [HUNGRY] on the enum `Panther_mood` will be removed. If these variants are still used in the database, this will fail.".into(),
                "The values [HUNGRY] on the enum `Tiger_mood` will be removed. If these variants are still used in the database, this will fail.".into(),]
            );
    };

    api.assert_schema().assert_tables_count(5).unwrap();
}

#[test_connector(capabilities(Enums))]
fn changing_all_values_of_enums_used_in_defaults_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            morningMood             CatMood @default(HUNGRY)
            alternateMorningMood    CatMood @default(HUNGRY)
            afternoonMood           CatMood @default(HAPPY)
            eveningMood             CatMood @default(HUNGRY)
            defaultMood             CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    let dm2 = r#"
        model Cat {
            id Int @id
            morningMood             CatMood @default(MEOW)
            alternateMorningMood    CatMood @default(MEOWMEOWMEOW)
            afternoonMood           CatMood @default(PURR)
            eveningMood             CatMood @default(MEOWMEOW)
            defaultMood             CatMood
        }

        enum CatMood {
            MEOW
            MEOWMEOW
            MEOWMEOWMEOW
            PURR
        }
    "#;

    api.schema_push(dm2).force(true).send_sync();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("eveningMood", |col| Ok(col.assert_enum_default("MEOWMEOW")))
        })
        .unwrap();
}

#[test_connector(tags(Postgres))]
fn existing_enums_are_picked_up(api: TestApi) {
    let sql = r#"
        CREATE TYPE "Genre" AS ENUM ('SKA', 'PUNK');

        CREATE TABLE "prisma-tests"."Band" (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            genre "Genre" NOT NULL
        );
    "#;

    api.raw_cmd(sql);

    let dm = r#"
        enum Genre {
            SKA
            PUNK
        }

        model Band {
            id Int @id @default(autoincrement())
            name String
            genre Genre
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang().assert_no_steps();
}
