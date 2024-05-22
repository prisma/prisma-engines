use indoc::{formatdoc, indoc};
use pretty_assertions::assert_eq;
use sql_migration_tests::test_api::*;
use std::io::Write;
use user_facing_errors::{schema_engine::ApplyMigrationError, UserFacingError};

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

#[test_connector(tags(Mssql, Postgres), preview_features("multiSchema"), namespaces("one", "two"))]
fn multi_schema_applying_two_migrations_works(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"
        model Cat {
            id      Int @id
            name    String
            @@schema("one")
        }
    "#,
        &[("schemas", "[\"one\", \"two\"]")],
        &["multiSchema"],
    );

    let migrations_directory = api.create_migrations_directory();

    api.create_migration("initial", &dm1, &migrations_directory).send_sync();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"
        model Cat {
            id          Int @id
            name        String
            @@schema("two")
        }
    "#,
        &[("schemas", "[\"one\", \"two\"]")],
        &["multiSchema"],
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

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("one", "two"))]
fn multi_schema_two_migrations_drop_fks(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"
        model First {
            id      Int @id
            name    String

            r1_second Second? @relation("r1")

            r2_second Second? @relation("r2", fields: [r2_secondId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r2_secondId Int? @unique

            @@schema("one")
        }
        model Second {
            id      Int @id
            name    String

            r1_first First @relation("r1", fields: [r1_firstId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r1_firstId Int @unique

            r2_first First? @relation("r2")

            @@schema("two")
        }
    "#,
        &[("schemas", "[\"one\", \"two\"]")],
        &["multiSchema"],
    );

    let migrations_directory = api.create_migrations_directory();

    api.create_migration("initial", &dm1, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    api.raw_cmd("INSERT INTO one.First (id, name, r2_secondId) VALUES(1, 'first', NULL)");
    api.raw_cmd("INSERT INTO two.Second (id, name, r1_firstId) VALUES(1, 'second', 1)");
    api.raw_cmd("INSERT INTO one.First (id, name, r2_secondId) VALUES(2, 'other', 1)");

    let dm2 = api.datamodel_with_provider_and_features(r#"
        model First {
            id      Int @id
            name    String

            r1_second Second? @relation("r1")

            r2_second Second? @relation("r2", fields: [r2_secondId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r2_secondId Int? @unique

            @@schema("two")
        }
        model Second {
            id      Int @id
            name    String

            r1_first First @relation("r1", fields: [r1_firstId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r1_firstId Int @unique

            r2_first First? @relation("r2")

            @@schema("one")
        }
      "#, &[("schemas", "[\"one\", \"two\"]")], &["multiSchema"]);

    api.create_migration("second-migration", &dm2, &migrations_directory)
        .send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["second-migration"]);

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&[]);
}

#[test_connector(tags(Mssql), preview_features("multiSchema"), namespaces("one", "two"))]
fn multi_schema_two_migrations_reset(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"
        model First {
            id      Int @id
            name    String

            r1_second Second? @relation("r1")

            r2_second Second? @relation("r2", fields: [r2_secondId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r2_secondId Int? @unique

            @@schema("one")
        }
        model Second {
            id      Int @id
            name    String

            r1_first First @relation("r1", fields: [r1_firstId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r1_firstId Int @unique

            r2_first First? @relation("r2")

            @@schema("two")
        }
    "#,
        &[("schemas", "[\"one\", \"two\"]")],
        &["multiSchema"],
    );

    let migrations_directory = api.create_migrations_directory();

    api.create_migration("initial", &dm1, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    api.raw_cmd("INSERT INTO one.First (id, name, r2_secondId) VALUES(1, 'first', NULL)");
    api.raw_cmd("INSERT INTO two.Second (id, name, r1_firstId) VALUES(1, 'second', 1)");
    api.raw_cmd("INSERT INTO one.First (id, name, r2_secondId) VALUES(2, 'other', 1)");

    let dm2 = api.datamodel_with_provider_and_features(r#"
        model First {
            id      Int @id
            name    String

            r1_second Second? @relation("r1")

            r2_second Second? @relation("r2", fields: [r2_secondId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r2_secondId Int? @unique

            @@schema("two")
        }
        model Second {
            id      Int @id
            name    String

            r1_first First @relation("r1", fields: [r1_firstId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            r1_firstId Int @unique

            r2_first First? @relation("r2")

            @@schema("one")
        }
      "#, &[("schemas", "[\"one\", \"two\"]")], &["multiSchema"]);

    api.create_migration("second-migration", &dm2, &migrations_directory)
        .send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["second-migration"]);

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&[]);

    let mut vec = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec);
    api.reset().send_sync(namespaces.clone());

    api.assert_schema_with_namespaces(namespaces)
        .assert_has_no_table("First")
        .assert_has_no_table("Second");
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
                t if t.contains(Tags::CockroachDb) => indoc! {r#"
                    ERROR: at or near "^": syntax error
                    DETAIL: source SQL:
                    SELECT (^.^)_n
                            ^
                    HINT: try \h SELECT
                "#},
                t if t.contains(Tags::Vitess) => "syntax error at position 10",
                t if t.contains(Tags::Mariadb) => "You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'^.^)_n\' at line 1",
                t if t.contains(Tags::Mysql) => "You have an error in your SQL syntax; check the manual that corresponds to your MySQL server version for the right syntax to use near \'^.^)_n\' at line 1",
                t if t.contains(Tags::Mssql) => "Incorrect syntax near \'^\'.",
                t if t.contains(Tags::Postgres) => "ERROR: syntax error at or near \"^\"",
                t if t.contains(Tags::Sqlite) => "unrecognized token: \"^\" in \n\nSELECT (^.^)_n;\n at offset 10",
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

    let mut migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

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

    let initial_path = api
        .create_migration("initial", &dm1, &migrations_directory)
        .send_sync()
        .migration_script_path();

    api.apply_migrations(&migrations_directory).send_sync();

    let mut file = std::fs::OpenOptions::new().append(true).open(initial_path).unwrap();
    file.write_all(b"-- this is just a harmless comment\nSELECT 1;")
        .unwrap();

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
        user_facing_errors::schema_engine::DatabaseSchemaNotEmpty::ERROR_CODE
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

#[test_connector]
fn applying_a_single_migration_multi_file_should_work(api: TestApi) {
    let schema_a = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );
    let schema_b = indoc::indoc! {r#"
        model Dog {
            id Int @id
            name String
        }
    "#};

    let dir = api.create_migrations_directory();

    api.create_migration_multi_file(
        "init",
        &[("schema_a.prisma", schema_a.as_str()), ("schema_b.prisma", schema_b)],
        &dir,
    )
    .send_sync();

    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["init"]);

    api.apply_migrations(&dir).send_sync().assert_applied_migrations(&[]);
}
