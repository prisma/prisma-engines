use sql_migration_tests::test_api::*;

#[test_connector]
fn adding_a_unique_constraint_should_warn(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    {
        api.insert("Test")
            .value("id", "abc")
            .value("name", "george")
            .result_raw();

        api.insert("Test")
            .value("id", "def")
            .value("name", "george")
            .result_raw();
    }

    let dm2 = r#"
        model Test {
            id String @id
            name String @unique
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(false)
        .send()
        .assert_warnings(&["A unique constraint covering the columns `[name]` on the table `Test` will be added. If there are existing duplicate values, this will fail.".into()]);

    api.dump_table("Test")
        .assert_row(0, |row| {
            row.assert_text_value("id", "abc").assert_text_value("name", "george")
        })
        .assert_row(1, |row| {
            row.assert_text_value("id", "def").assert_text_value("name", "george")
        });
}

// Excluding Vitess because schema changes being asynchronous messes with our assertions
// (dump_table).
#[test_connector(tags(Mysql, Postgres), exclude(Vitess))]
fn dropping_enum_values_should_warn(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name Test_name
        }

        enum Test_name{
            george
            paul
            ringo
            john
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    {
        api.insert("Test")
            .value("id", "abc")
            .value("name", "george")
            .result_raw();

        api.insert("Test")
            .value("id", "def")
            .value("name", "george")
            .result_raw();
    }

    let dm2 = r#"
        model Test {
            id String @id
            name Test_name
        }

        enum Test_name{
            paul
            ringo
            john
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(false)
        .send()
        .assert_warnings(&["The values [george] on the enum `Test_name` will be removed. If these variants are still used in the database, this will fail.".into()]);

    api.dump_table("Test")
        .assert_row(0, |row| {
            row.assert_text_value("id", "abc").assert_text_value("name", "george")
        })
        .assert_row(1, |row| {
            row.assert_text_value("id", "def").assert_text_value("name", "george")
        });
}

#[test_connector]
fn adding_a_unique_constraint_when_existing_data_respects_it_works(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw();

    api.insert("Test")
        .value("id", "def")
        .value("name", "georgina")
        .result_raw();

    let dm2 = r#"
        model Test {
            id String @id
            name String @unique
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_warnings(&["A unique constraint covering the columns `[name]` on the table `Test` will be added. If there are existing duplicate values, this will fail.".into()]);

    api.dump_table("Test")
        .assert_row(0, |row| {
            row.assert_text_value("id", "abc").assert_text_value("name", "george")
        })
        .assert_row(1, |row| {
            row.assert_text_value("id", "def").assert_text_value("name", "georgina")
        });
}
