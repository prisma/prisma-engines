use quaint::Value;
use sql_migration_tests::test_api::*;

#[test_connector]
fn altering_the_type_of_a_column_in_a_non_empty_table_warns(api: TestApi) {
    let dm1 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs BigInt
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let insert = quaint::ast::Insert::single_into(api.render_table_name("User"))
        .value("id", "abc")
        .value("name", "Shinzo")
        .value("dogs", 7);

    api.query(insert.into());

    let dm2 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_warnings(&[
        if api.is_cockroach() {
            "You are about to alter the column `dogs` on the `User` table, which contains 1 non-null values. The data in that column will be cast from `Int8` to `Int4`.".into()
        } else if api.is_postgres() {
            "You are about to alter the column `dogs` on the `User` table, which contains 1 non-null values. The data in that column will be cast from `BigInt` to `Integer`.".into()
        } else if api.lower_cases_table_names() {
            "You are about to alter the column `dogs` on the `user` table, which contains 1 non-null values. The data in that column will be cast from `BigInt` to `Int`.".into()
        } else {
            "You are about to alter the column `dogs` on the `User` table, which contains 1 non-null values. The data in that column will be cast from `BigInt` to `Int`.".into()
        }
    ]);

    api.dump_table("User")
        .assert_single_row(|row| row.assert_int_value("dogs", 7));

    api.assert_schema().assert_table("User", |table| {
        table.assert_column("dogs", |col| col.assert_type_is_bigint().assert_is_required())
    });
}

#[test_connector]
fn migrating_a_required_column_from_int_to_string_should_cast(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            serialNumber Int
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Test")
        .value("id", "abcd")
        .value("serialNumber", 47i64)
        .result_raw();

    api.dump_table("Test")
        .assert_single_row(|row| row.assert_text_value("id", "abcd").assert_int_value("serialNumber", 47));

    let dm2 = r#"
        model Test {
            id String @id
            serialNumber String
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("serialNumber", |col| col.assert_type_is_string())
    });

    api.dump_table("Test").assert_single_row(|row| {
        row.assert_text_value("id", "abcd")
            .assert_text_value("serialNumber", "47")
    });
}

#[test_connector(capabilities(ScalarLists))]
fn changing_a_string_array_column_to_scalar_is_fine(api: TestApi) {
    let dm1 = r#"
        model Film {
            id String @id
            mainProtagonist String[]
        }
        "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Film")
        .value("id", "film1")
        .value(
            "mainProtagonist",
            Value::array(vec![Value::text("giant shark"), Value::text("jason statham")]),
        )
        .result_raw();

    let dm2 = r#"
            model Film {
                id String @id
                mainProtagonist String
            }
            "#;

    api.schema_push_w_datasource(dm2).force(true).send().assert_green();

    api.assert_schema().assert_table("Film", |table| {
        table.assert_column("mainProtagonist", |column| column.assert_is_required())
    });

    api.dump_table("Film").assert_single_row(|row| {
        row.assert_text_value("id", "film1")
            // the array got cast to a string by postgres
            .assert_text_value("mainProtagonist", "{\"giant shark\",\"jason statham\"}")
    });
}

#[test_connector(capabilities(ScalarLists))]
fn changing_an_int_array_column_to_scalar_is_not_possible(api: TestApi) {
    let dm1 = r#"
        model Film {
            id String @id
            mainProtagonist Int[]
        }
        "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Film")
        .value("id", "film1")
        .value("mainProtagonist", Value::array(vec![Value::int32(7), Value::int32(11)]))
        .result_raw();

    let dm2 = r#"
            model Film {
                id String @id
                mainProtagonist Int
            }
            "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_no_warning()
        .assert_unexecutable(&["Changed the type of `mainProtagonist` on the `Film` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.".into()]);

    api.assert_schema().assert_table("Film", |table| {
        table.assert_column("mainProtagonist", |column| column.assert_is_list())
    });

    api.dump_table("Film").assert_single_row(|row| {
        row.assert_text_value("id", "film1")
            .assert_array_value("mainProtagonist", &[7.into(), 11.into()])
    });
}

#[test_connector(exclude(CockroachDb))]
fn int_to_string_conversions_work(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id  Int @id @default(autoincrement())
            tag Int
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Cat").value("tag", 20).result_raw();

    let dm2 = r#"
        model Cat {
            id  Int @id @default(autoincrement())
            tag String
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    if api.is_vitess() {
        return; // asynchronous migrations mess with the following assertion
    }

    api.dump_table("Cat")
        .assert_single_row(|row| row.assert_text_value("tag", "20"));
}

#[test_connector(exclude(CockroachDb))]
fn string_to_int_conversions_are_risky(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id  Int @id @default(autoincrement())
            tag String
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Cat").value("tag", "20").result_raw();

    let dm2 = r#"
        model Cat {
            id  Int @id @default(autoincrement())
            tag Int
        }
    "#;

    if api.is_postgres() {
        // Not executable
        api.schema_push_w_datasource(dm2)
            .force(true)
            .send()
            .assert_no_warning()
            .assert_unexecutable(&["Changed the type of `tag` on the `Cat` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.".into()]);
    } else if api.is_mysql() {
        // Executable, conditionally.
        if api.lower_cases_table_names() {
            api.schema_push_w_datasource(dm2)
            .force(true)
            .send()
            .assert_warnings(&[
                "You are about to alter the column `tag` on the `cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Int`.".into()
            ])
            .assert_executable()
            .assert_has_executed_steps();

            api.dump_table("Cat")
                .assert_single_row(|row| row.assert_int_value("tag", 20));
        } else {
            // Executable, conditionally.
            api.schema_push_w_datasource(dm2)
                .force(true)
                .send()
                .assert_warnings(&[
                    "You are about to alter the column `tag` on the `Cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Int`.".into()
                ])
                .assert_executable()
                .assert_has_executed_steps();

            api.dump_table("Cat")
                .assert_single_row(|row| row.assert_int_value("tag", 20));
        }
    } else if api.is_mssql() {
        api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_warnings(&[
            "You are about to alter the column `tag` on the `Cat` table, which contains 1 non-null values. The data in that column will be cast from `NVarChar(1000)` to `Int`.".into()
        ])
        .assert_executable()
        .assert_has_executed_steps();

        api.dump_table("Cat")
            .assert_single_row(|row| row.assert_int_value("tag", 20));
    } else if api.is_sqlite() {
        api.schema_push_w_datasource(dm2)
            .force(true)
            .send()
            .assert_warnings(&[
                "You are about to alter the column `tag` on the `Cat` table, which contains 1 non-null values. The data in that column will be cast from `String` to `Int`.".into()
            ])
            .assert_executable()
            .assert_has_executed_steps();

        api.dump_table("Cat")
            .assert_single_row(|row| row.assert_int_value("tag", 20));
    }
}

// of course, 2018-01-18T08:01:02Z gets cast to 20180118080102.0 on MySQL
// of course, 2018-01-18T08:01:02Z gets cast to 1516262462000.0 (UNIX timestamp) on SQLite and Cockroach
#[test_connector(exclude(Mysql, Sqlite, CockroachDb))]
fn datetime_to_float_conversions_are_impossible(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id          Int @id @default(autoincrement())
            birthday    DateTime
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Cat")
        .value("birthday", Value::datetime("2018-01-18T08:01:02Z".parse().unwrap()))
        .result_raw();

    let dm2 = r#"
        model Cat {
            id          Int @id @default(autoincrement())
            birthday    Float @default(3.0)
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_warnings(&[
            "The `birthday` column on the `Cat` table would be dropped and recreated. This will lead to data loss."
                .into(),
        ])
        .assert_executable()
        .assert_no_steps();

    api.dump_table("Cat")
        .assert_single_row(|row| row.assert_datetime_value("birthday", "2018-01-18T08:01:02Z".parse().unwrap()));

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_warnings(&[
            "The `birthday` column on the `Cat` table would be dropped and recreated. This will lead to data loss."
                .into(),
        ])
        .assert_executable()
        .assert_has_executed_steps();

    api.dump_table("Cat")
        .assert_single_row(|row| row.assert_float_value("birthday", 3.0));
}
