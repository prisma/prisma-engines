use indoc::formatdoc;
use migration_engine_tests::test_api::*;
use pretty_assertions::assert_eq;
use user_facing_errors::{migration_engine::ApplyMigrationError, UserFacingError};

#[test_connector]
fn apply_migrations_with_an_empty_migrations_folder_works(api: TestApi) {
    let dir = api.create_migrations_directory();

    api.apply_migrations(&dir).send_sync().assert_applied_migrations(&[]);
}

#[test_connector]
fn applying_a_single_migration_should_work(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("init", &dm, &dir).send_sync();

    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["init"]);

    api.apply_migrations(&dir).send_sync().assert_applied_migrations(&[]);
}

#[test_connector]
fn applying_two_migrations_works(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let migrations_directory = api.create_migrations_directory();

    api.create_migration("initial", &dm1, &migrations_directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &migrations_directory)
        .send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&[]);
}

#[test_connector]
fn migrations_should_fail_when_the_script_is_invalid(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let migrations_directory = api.create_migrations_directory();

    api.create_migration("initial", &dm1, &migrations_directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let second_migration_name = api
        .create_migration("second-migration", &dm2, &migrations_directory)
        .send_sync()
        .modify_migration(|contents| contents.push_str("\nSELECT (^.^)_n;\n"))
        .into_output()
        .generated_migration_name
        .unwrap();

    let error = api
        .apply_migrations(&migrations_directory)
        .send_unwrap_err()
        .to_user_facing()
        .unwrap_known();

    // Assertions about the user facing error.
    {
        let expected_error_message = formatdoc!(
            r#"
                A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

                Migration name: {second_migration_name}

                Database error code: {error_code}

                Database error:
                {message}
                "#,
            second_migration_name = second_migration_name,
            error_code = match api.tags() {
                t if t.contains(Tags::Vitess) => 1105,
                t if t.contains(Tags::Mysql) => 1064,
                t if t.contains(Tags::Mssql) => 102,
                t if t.contains(Tags::Postgres) => 42601,
                t if t.contains(Tags::Sqlite) => 1,
                _ => todo!(),
            },
            message = match api.tags() {
                t if t.contains(Tags::Vitess) => "syntax error at position 10",
                t if t.contains(Tags::Mariadb) => "You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'^.^)_n\' at line 1",
                t if t.contains(Tags::Mysql) => "You have an error in your SQL syntax; check the manual that corresponds to your MySQL server version for the right syntax to use near \'^.^)_n\' at line 1",
                t if t.contains(Tags::Mssql) => "Incorrect syntax near \'^\'.",
                t if t.contains(Tags::Postgres) => "ERROR: syntax error at or near \"^\"",
                t if t.contains(Tags::Sqlite) => "unrecognized token: \"^\"",
                _ => todo!(),
            },
        );

        assert_eq!(error.error_code, ApplyMigrationError::ERROR_CODE);
        assert!(
            error.message.starts_with(&expected_error_message),
            "Actual:\n{}\n\nExpected:\n{}",
            error.message,
            expected_error_message
        );
    }

    let mut migrations = api
        .block_on(api.migration_persistence().list_migrations())
        .unwrap()
        .unwrap();

    assert_eq!(migrations.len(), 2);

    let second = migrations.pop().unwrap();
    let first = migrations.pop().unwrap();

    first
        .assert_migration_name("initial")
        .assert_applied_steps_count(1)
        .assert_success();

    second
        .assert_migration_name("second-migration")
        .assert_applied_steps_count(0)
        .assert_failed();
}

#[test_connector]
fn migrations_should_not_reapply_modified_migrations(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let migrations_directory = api.create_migrations_directory();

    let assertions = api.create_migration("initial", &dm1, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory).send_sync();

    assertions.modify_migration(|script| *script = format!("/* this is just a harmless comment */\n{}", script));

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &migrations_directory)
        .send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["second-migration"]);
}

#[test_connector]
fn migrations_should_fail_on_an_uninitialized_nonempty_database(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.schema_push(dm.clone()).send().assert_green();

    let directory = api.create_migrations_directory();

    api.create_migration("01-init", &dm, &directory)
        .send_sync()
        .assert_migration_directories_count(1);

    let known_error = api
        .apply_migrations(&directory)
        .send_unwrap_err()
        .to_user_facing()
        .unwrap_known();

    assert_eq!(
        known_error.error_code,
        user_facing_errors::migration_engine::DatabaseSchemaNotEmpty::ERROR_CODE
    );
}

// Reference for the tables created by PostGIS: https://postgis.net/docs/manual-1.4/ch04.html#id418599
#[test_connector(tags(Postgres))]
fn migrations_should_succeed_on_an_uninitialized_nonempty_database_with_postgis_tables(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let create_spatial_ref_sys_table = "CREATE TABLE IF NOT EXISTS \"spatial_ref_sys\" ( id SERIAL PRIMARY KEY )";
    // The capitalized Geometry is intentional here, because we want the matching to be case-insensitive.
    let create_geometry_columns_table = "CREATE TABLE IF NOT EXiSTS \"Geometry_columns\" ( id SERIAL PRIMARY KEY )";

    api.raw_cmd(create_spatial_ref_sys_table);
    api.raw_cmd(create_geometry_columns_table);

    let directory = api.create_migrations_directory();

    api.create_migration("01-init", &dm, &directory)
        .send_sync()
        .assert_migration_directories_count(1);

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["01-init"]);
}
