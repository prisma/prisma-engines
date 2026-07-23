mod sql_unexecutable_migrations;
mod sqlite_existing_data_tests;
mod type_migration_tests;

use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use quaint::ast::*;
use sql_migration_tests::test_api::*;
use sql_schema_describer::DefaultKind;

#[test_connector]
fn dropping_a_table_with_rows_should_warn(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::single_into(api.render_table_name("Test")).value("id", "test");

    api.query(insert.into());

    let dm = "";

    let warn = format!(
        "You are about to drop the `{}` table, which is not empty (1 rows).",
        api.normalize_identifier("Test")
    );

    api.schema_push_w_datasource(dm)
        .send()
        .assert_warnings(&[warn.into()])
        .assert_no_steps();
}

#[test_connector]
fn dropping_a_table_with_rows_multi_file_should_warn(api: TestApi) {
    let schema_a = r#"
        model Cat {
            id String @id @default(cuid())
        }
    "#;
    let schema_b = r#"
        model Dog {
            id String @id @default(cuid())
        }
    "#;

    api.schema_push_w_datasource_multi_file(&[("a.prisma", schema_a), ("b.prisma", schema_b)])
        .send()
        .assert_green();

    api.query(
        Insert::single_into(api.render_table_name("Cat"))
            .value("id", "test")
            .into(),
    );
    api.query(
        Insert::single_into(api.render_table_name("Dog"))
            .value("id", "test")
            .into(),
    );

    let warn = format!(
        "You are about to drop the `{}` table, which is not empty (1 rows).",
        api.normalize_identifier("Dog")
    );

    api.schema_push_w_datasource_multi_file(&[("a.prisma", schema_a)])
        .send()
        .assert_warnings(&[warn.into()])
        .assert_no_steps();
}

#[test_connector]
fn dropping_a_column_with_non_null_values_should_warn(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            puppiesCount Int?
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Test"), ["id", "puppiesCount"])
        .values(("a", 7))
        .values(("b", 8));

    api.query(insert.into());

    // Drop the `favouriteAnimal` column.
    let dm = r#"
            model Test {
                id String @id @default(cuid())
            }
        "#;

    let warn = format!(
        "You are about to drop the column `puppiesCount` on the `{}` table, which still contains 2 non-null values.",
        api.normalize_identifier("Test")
    );

    api.schema_push_w_datasource(dm)
        .send()
        .assert_warnings(&[warn.into()])
        .assert_no_steps();
}

#[test_connector]
fn altering_a_column_without_non_null_values_should_not_warn(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            puppiesCount Int?
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Test"), ["id"])
        .values(("a",))
        .values(("b",));

    api.query(insert.into());

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            puppiesCount Float?
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_warnings(&[])
        .assert_has_executed_steps();
}

#[test_connector]
fn altering_a_column_with_non_null_values_should_warn(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age String?
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Test"), vec!["id", "age"])
        .values(("a", 12))
        .values(("b", 22));

    api.query(insert.into());

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Int?
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_warnings(&[
        if api.is_postgres() {
             "The `age` column on the `Test` table would be dropped and recreated. This will lead to data loss.".into()
        } else if api.is_mssql() {
             "You are about to alter the column `age` on the `Test` table, which contains 2 non-null values. The data in that column will be cast from `NVarChar(1000)` to `Int`.".into()

        } else if api.is_mysql() {
            if api.lower_cases_table_names() {

                 "You are about to alter the column `age` on the `test` table, which contains 2 non-null values. The data in that column will be cast from `VarChar(191)` to `Int`.".into()
            } else {

                 "You are about to alter the column `age` on the `Test` table, which contains 2 non-null values. The data in that column will be cast from `VarChar(191)` to `Int`.".into()
            }
        } else {
             "You are about to alter the column `age` on the `Test` table, which contains 2 non-null values. The data in that column will be cast from `String` to `Int`.".into()

        }
    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    ]).assert_no_steps();

    assert_eq!(api.dump_table("Test").len(), 2);
}

#[test_connector(exclude(CockroachDb))]
fn column_defaults_can_safely_be_changed(api: TestApi) {
    let combinations = &[
        ("Meow", Some(PrismaValue::String("Cats".to_string())), None),
        ("Freedom", None, Some(PrismaValue::String("Braveheart".to_string()))),
        (
            "OutstandingMovies",
            Some(PrismaValue::String("Cats".to_string())),
            Some(PrismaValue::String("Braveheart".to_string())),
        ),
    ];

    for (model_name, first_default, second_default) in combinations {
        let span = tracing::info_span!("Combination", model_name, ?first_default, ?second_default);
        let _combination_scope = span.enter();
        tracing::info!("Testing new combination");

        // Set up the initial schema
        {
            let dm1 = format!(
                r#"
                    model {} {{
                        id String @id
                        name String? {}
                    }}
                "#,
                model_name,
                first_default
                    .as_ref()
                    .map(|default| format!("@default(\"{default}\")"))
                    .unwrap_or_else(String::new)
            );

            api.schema_push_w_datasource(dm1).force(true).send();

            api.assert_schema().assert_table(model_name, |table| {
                table.assert_column("name", |column| {
                    if let Some(first_default) = first_default.as_ref() {
                        column.assert_default_value(first_default)
                    } else {
                        column.assert_has_no_default()
                    }
                })
            });
        }

        // Insert data
        {
            let insert_span = tracing::info_span!("Data insertion");
            let _insert_scope = insert_span.enter();

            let query = Insert::single_into(api.render_table_name(model_name)).value("id", "abc");

            api.query(query.into());

            let query = Insert::single_into(api.render_table_name(model_name))
                .value("id", "def")
                .value("name", "Waterworld");

            api.query(query.into());

            let data = api.dump_table(model_name);
            let names: Vec<PrismaValue> = data
                .into_iter()
                .filter_map(|row| {
                    row.get("name")
                        .map(|val| val.to_string().map(PrismaValue::String).unwrap_or(PrismaValue::Null))
                })
                .collect();

            assert_eq!(
                &[
                    first_default.as_ref().cloned().unwrap_or(PrismaValue::Null),
                    PrismaValue::String("Waterworld".to_string())
                ],
                names.as_slice()
            );
        }

        // Migrate
        {
            let dm2 = format!(
                r#"
                    model {} {{
                        id String @id
                        name String? {}
                    }}
                "#,
                model_name,
                second_default
                    .as_ref()
                    .map(|default| format!(r#"@default("{default}")"#))
                    .unwrap_or_else(String::new)
            );

            api.schema_push_w_datasource(dm2).send().assert_green();
        }

        // Check that the data is still there
        {
            let data = api.dump_table(model_name);
            let names: Vec<PrismaValue> = data
                .into_iter()
                .filter_map(|row| {
                    row.get("name")
                        .map(|val| val.to_string().map(PrismaValue::String).unwrap_or(PrismaValue::Null))
                })
                .collect();
            assert_eq!(
                &[
                    first_default.as_ref().cloned().unwrap_or(PrismaValue::Null),
                    PrismaValue::String("Waterworld".to_string())
                ],
                names.as_slice()
            );

            api.assert_schema().assert_table(model_name, |table| {
                table.assert_column("name", |column| {
                    if let Some(second_default) = second_default.as_ref() {
                        column.assert_default_value(second_default)
                    } else {
                        column.assert_has_no_default()
                    }
                })
            });
        }
    }
}

#[test_connector]
fn changing_a_column_from_required_to_optional_should_work(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int @default(30)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Test"), ["id", "age"])
        .values(("a", 12))
        .values(("b", 22));

    api.query(insert.into());

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Int? @default(30)
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    // Check that no data was lost.
    {
        let data = api.dump_table("Test");
        assert_eq!(data.len(), 2);
        let ages: Vec<i64> = data
            .into_iter()
            .map(|row| row.get("age").unwrap().as_integer().unwrap())
            .collect();

        assert_eq!(ages, &[12, 22]);
    }
}

#[test_connector(exclude(Sqlite))]
fn changing_a_column_from_optional_to_required_is_unexecutable(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int?
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Test"), ["id", "age"])
        .values(("a", 12))
        .values(("b", 22))
        .values(("c", ValueType::Int32(None)));

    api.query(insert.into());

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Int @default(30)
        }
    "#;

    let error = format!(
        "Made the column `age` on table `{}` required, but there are 1 existing NULL values.",
        api.normalize_identifier("Test")
    );

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_no_warning()
        .assert_unexecutable(&[error])
        // The schema should not change because the migration should not run if there are warnings
        // and the force flag isn't passed.
        .assert_no_steps();

    // Check that no data was lost.
    {
        let data = api.dump_table("Test");
        assert_eq!(data.len(), 3);
        let ages: Vec<Option<i64>> = data
            .into_iter()
            .map(|row| row.get("age").unwrap().as_integer())
            .collect();

        assert_eq!(ages, &[Some(12), Some(22), None]);
    }
}

#[test_connector(exclude(Vitess))]
fn dropping_a_table_referenced_by_foreign_keys_must_work(api: TestApi) {
    use quaint::ast::*;

    let dm1 = r#"
        model Category {
            id Int @id
            name String
            r    Recipe[]
        }

        model Recipe {
            id Int @id
            categoryId Int
            category Category @relation(fields: [categoryId], references: [id])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        .assert_table("Category", |table| table.assert_columns_count(2))
        .assert_table("Recipe", |table| {
            table.assert_fk_on_columns(&["categoryId"], |fk| fk.assert_references("Category", &["id"]))
        });

    let id: i32 = 1;

    let insert = Insert::single_into(api.render_table_name("Category"))
        .value("name", "desserts")
        .value("id", id);
    api.query(insert.into());

    let insert = Insert::single_into(api.render_table_name("Recipe"))
        .value("categoryId", id)
        .value("id", id);
    api.query(insert.into());

    let dm2 = r#"
        model Recipe {
            id Int @id
        }
    "#;

    api.schema_push_w_datasource(dm2).force(true).send();

    api.assert_schema()
        .assert_tables_count(1)
        .assert_table("Recipe", |table| table.assert_foreign_keys_count(0));
}

#[test_connector]
fn string_columns_do_not_get_arbitrarily_migrated(api: TestApi) {
    use quaint::ast::*;

    let dm1 = r#"
        model User {
            id           String  @id @default(cuid())
            name         String?
            email        String  @unique
            kindle_email String? @unique
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let insert = Insert::single_into(api.render_table_name("User"))
        .value("id", "the-id")
        .value("name", "George")
        .value("email", "george@prisma.io")
        .value("kindle_email", "george+kindle@prisma.io");

    api.query(insert.into());

    let dm2 = r#"
        model User {
            id           String  @id @default(cuid())
            name         String?
            email        String  @unique
            kindle_email String? @unique
            count        Int     @default(0)
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green().assert_green();

    // Check that the string values are still there.
    let select = Select::from_table(api.render_table_name("User"))
        .column("name")
        .column("kindle_email")
        .column("email");

    let counts = api.query(select.into());

    let row = counts.get(0).unwrap();

    assert_eq!(row.get("name").unwrap().as_str().unwrap(), "George");
    assert_eq!(
        row.get("kindle_email").unwrap().as_str().unwrap(),
        "george+kindle@prisma.io"
    );
    assert_eq!(row.get("email").unwrap().as_str().unwrap(), "george@prisma.io");
}

#[test_connector]
fn altering_the_type_of_a_column_in_an_empty_table_should_not_warn(api: TestApi) {
    let dm1 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs String
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_column("dogs", |col| col.assert_type_is_string().assert_is_required())
    });
}

#[test_connector]
fn making_a_column_required_in_an_empty_table_should_not_warn(api: TestApi) {
    let dm1 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int?
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_column("dogs", |col| col.assert_type_is_int().assert_is_required())
    });
}

// Excluding Vitess because schema changes being asynchronous messes with our assertions
// (dump_table).
#[test_connector(tags(Mysql, Postgres), exclude(Vitess))]
fn enum_variants_can_be_added_without_data_loss(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id String @id
            mood Mood
        }

        model Human {
            id String @id
            mood Mood
        }

        enum Mood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1)
        .migration_id(Some("initial-setup"))
        .send()
        .assert_green();

    {
        let cat_inserts = quaint::ast::Insert::multi_into(api.render_table_name("Cat"), vec!["id", "mood"])
            .values((Value::text("felix"), Value::enum_variant("HUNGRY")))
            .values((Value::text("mittens"), Value::enum_variant("HAPPY")));

        api.query(cat_inserts.into());
    }

    let dm2 = r#"
        model Cat {
            id String @id
            mood Mood
        }

        model Human {
            id String @id
            mood Mood
        }

        enum Mood {
            HAPPY
            HUNGRY
            ABSOLUTELY_FABULOUS
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .migration_id(Some("add-absolutely-fabulous-variant"))
        .send()
        .assert_green();

    // Assertions
    {
        let cat_data = api.dump_table("Cat");
        let cat_data: Vec<Vec<quaint::ast::Value>> =
            cat_data.into_iter().map(|row| row.into_iter().collect()).collect();

        let expected_cat_data = &[
            &[
                Value::text("felix"),
                if api.is_mysql() {
                    Value::text("HUNGRY")
                } else {
                    Value::enum_variant("HUNGRY")
                },
            ],
            &[
                Value::text("mittens"),
                if api.is_mysql() {
                    Value::text("HAPPY")
                } else {
                    Value::enum_variant("HAPPY")
                },
            ],
        ];

        assert_eq!(cat_data, expected_cat_data);

        let human_data = api.dump_table("Human");
        let human_data: Vec<Vec<Value>> = human_data.into_iter().map(|row| row.into_iter().collect()).collect();
        let expected_human_data: Vec<Vec<Value>> = Vec::new();
        assert_eq!(human_data, expected_human_data);

        if api.is_mysql() {
            api.assert_schema()
                .assert_enum(&api.normalize_identifier("Cat_mood"), |enm| {
                    enm.assert_values(&["HAPPY", "HUNGRY", "ABSOLUTELY_FABULOUS"])
                })
                .assert_enum(&api.normalize_identifier("Human_mood"), |enm| {
                    enm.assert_values(&["HAPPY", "HUNGRY", "ABSOLUTELY_FABULOUS"])
                });
        } else {
            api.assert_schema().assert_enum("Mood", |enm| {
                enm.assert_values(&["HAPPY", "HUNGRY", "ABSOLUTELY_FABULOUS"])
            });
        };
    }
}

// Excluding Vitess because schema changes being asynchronous messes with our assertions
// (dump_table).
#[test_connector(tags(Mysql, Postgres), exclude(Vitess))]
fn enum_variants_can_be_dropped_without_data_loss(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id String @id
            mood Mood
        }

        model Human {
            id String @id
            mood Mood
        }

        enum Mood {
            OUTRAGED
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1)
        .migration_id(Some("initial-setup"))
        .send()
        .assert_green();

    {
        let cat_inserts = quaint::ast::Insert::multi_into(api.render_table_name("Cat"), ["id", "mood"])
            .values((Value::text("felix"), Value::enum_variant("HUNGRY")))
            .values((Value::text("mittens"), Value::enum_variant("HAPPY")));

        api.query(cat_inserts.into());
    }

    let dm2 = r#"
        model Cat {
            id String @id
            mood Mood
        }

        model Human {
            id String @id
            mood Mood
        }

        enum Mood {
            HAPPY
            HUNGRY
        }
    "#;

    let res = api
        .schema_push_w_datasource(dm2)
        .migration_id(Some("add-absolutely-fabulous-variant"))
        .force(true)
        .send();

    if api.is_mysql() {
        res.assert_warnings(&["The values [OUTRAGED] on the enum `Human_mood` will be removed. If these variants are still used in the database, this will fail.".into(), "The values [OUTRAGED] on the enum `Human_mood` will be removed. If these variants are still used in the database, this will fail.".into()]);
    } else {
        res.assert_warnings(&["The values [OUTRAGED] on the enum `Mood` will be removed. If these variants are still used in the database, this will fail.".into()]);
    }

    // Assertions
    {
        let cat_data = api.dump_table("Cat");
        let cat_data: Vec<Vec<quaint::ast::Value>> =
            cat_data.into_iter().map(|row| row.into_iter().collect()).collect();

        let expected_cat_data = &[
            &[
                Value::text("felix"),
                if api.is_mysql() {
                    Value::text("HUNGRY")
                } else {
                    Value::enum_variant("HUNGRY")
                },
            ],
            &[
                Value::text("mittens"),
                if api.is_mysql() {
                    Value::text("HAPPY")
                } else {
                    Value::enum_variant("HAPPY")
                },
            ],
        ];

        assert_eq!(cat_data, expected_cat_data);

        let human_data = api.dump_table("Human");
        let human_data: Vec<Vec<Value>> = human_data.into_iter().map(|row| row.into_iter().collect()).collect();
        let expected_human_data: Vec<Vec<Value>> = Vec::new();
        assert_eq!(human_data, expected_human_data);

        if api.is_mysql() {
            api.assert_schema()
                .assert_enum(&api.normalize_identifier("Cat_mood"), |enm| {
                    enm.assert_values(&["HAPPY", "HUNGRY"])
                })
                .assert_enum(&api.normalize_identifier("Human_mood"), |enm| {
                    enm.assert_values(&["HAPPY", "HUNGRY"])
                });
        } else {
            api.assert_schema()
                .assert_enum("Mood", |enm| enm.assert_values(&["HAPPY", "HUNGRY"]));
        };
    }
}

#[test_connector]
fn set_default_current_timestamp_on_existing_column_works(api: TestApi) {
    let dm1 = r#"
        model User {
            id BigInt @id
            created_at DateTime
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let insert = Insert::single_into(api.render_table_name("User"))
        .value("id", 5)
        .value("created_at", Value::datetime("2020-06-15T14:50:00Z".parse().unwrap()));
    api.query(insert.into());

    let dm2 = r#"
        model User {
            id BigInt @id
            created_at DateTime @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_no_warning();

    api.assert_schema().assert_table("User", |table| {
        table.assert_column("created_at", |column| {
            column.assert_default_kind(Some(DefaultKind::Now))
        })
    });
}

// Excluding Vitess because schema changes being asynchronous messes with our assertions
// (dump_table).
// exclude: there is a cockroach-specific test. It's unexecutable there.
#[test_connector(exclude(CockroachDb, Vitess))]
fn primary_key_migrations_do_not_cause_data_loss(api: TestApi) {
    let dm1 = r#"
        model Dog {
            name            String
            passportNumber  Int
            p               Puppy[]

            @@id([name, passportNumber])
        }

        model Puppy {
            id String @id
            motherName String
            motherPassportNumber Int
            mother Dog @relation(fields: [motherName, motherPassportNumber], references: [name, passportNumber])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Dog")
        .value("name", "Marnie")
        .value("passportNumber", 8000)
        .result_raw();

    api.insert("Puppy")
        .value("id", "12345")
        .value("motherName", "Marnie")
        .value("motherPassportNumber", 8000)
        .result_raw();

    // Make Dog#passportNumber a String.
    let dm2 = r#"
        model Dog {
            name           String
            passportNumber String
            p              Puppy[]


            @@id([name, passportNumber])
        }

        model Puppy {
            id String @id
            motherName String
            motherPassportNumber String
            mother Dog @relation(fields: [motherName, motherPassportNumber], references: [name, passportNumber])
        }
    "#;

    let warn = format!(
        "The primary key for the `{}` table will be changed. If it partially fails, the table could be left without primary key constraint.",
        api.normalize_identifier("Dog"),
    );

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_warnings(&[warn.into()]);

    api.assert_schema().assert_table("Dog", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["name", "passportNumber"]))
    });

    let dog = api.dump_table("Dog");
    let dog_row: Vec<quaint::Value> = dog.into_single().unwrap().into_iter().collect();

    assert_eq!(dog_row, &[Value::text("Marnie"), Value::text("8000")]);

    let puppy = api.dump_table("Puppy");

    let puppy_row: Vec<quaint::Value> = puppy.into_single().unwrap().into_iter().collect();

    assert_eq!(
        puppy_row,
        &[Value::text("12345"), Value::text("Marnie"), Value::text("8000")]
    );
}
