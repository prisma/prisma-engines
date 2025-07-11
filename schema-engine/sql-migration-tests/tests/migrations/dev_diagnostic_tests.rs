use pretty_assertions::assert_eq;
use schema_core::{json_rpc::types::*, schema_api};
use sql_migration_tests::{test_api::*, utils::list_migrations};
use std::io::Write;
use user_facing_errors::{schema_engine::MigrationDoesNotApplyCleanly, UserFacingError};

trait DevActionExt {
    fn is_create_migration(&self) -> bool;
    fn as_reset(&self) -> Option<&str>;
}

impl DevActionExt for DevAction {
    fn is_create_migration(&self) -> bool {
        matches!(self, DevAction::CreateMigration)
    }

    fn as_reset(&self) -> Option<&str> {
        match self {
            DevAction::Reset(rst) => Some(&rst.reason),
            _ => None,
        }
    }
}

#[test_connector]
fn dev_diagnostic_on_an_empty_database_without_migration_returns_create_migration(api: TestApi) {
    let directory = api.create_migrations_directory();
    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    assert!(action.is_create_migration());
}

#[test_connector]
fn dev_diagnostic_after_two_migrations_happy_path(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    assert!(action.is_create_migration());
}

#[test_connector]
fn dev_diagnostic_detects_drift(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.schema_push_w_datasource(dm2).send();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    let expected_start = "Drift detected: Your database schema is not in sync with your migration history.";
    assert!(action.as_reset().unwrap().starts_with(expected_start));
}

#[test_connector(exclude(Postgres, Mssql))]
fn dev_diagnostic_calculates_drift_in_presence_of_failed_migrations(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("01_initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }

        model Dog {
            id Int @id
            fluffiness Float
        }
    "#,
    );

    let mut original_migration = String::new();
    let (migration_two_name, migration_two_path) = {
        let out = api
            .create_migration("02_add_dogs", &dm2, &directory)
            .send_sync()
            .modify_migration(|migration| {
                original_migration.push_str(migration);
                migration.push_str("\nSELECT YOLO;");
            });
        let path = out.migration_script_path();
        (out.into_output().generated_migration_name, path)
    };

    let err = api.apply_migrations(&directory).send_unwrap_err().to_string();
    assert!(err.contains("yolo") || err.contains("YOLO"), "{}", err);

    std::fs::write(migration_two_path, original_migration.as_bytes()).unwrap();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    let expected_message = format!(
        "- The migration `{migration_two_name}` failed.\n- The migration `{migration_two_name}` was modified after it was applied.\n- Drift detected: Your database schema is not in sync with your migration history.\n",
    );

    assert!(action.as_reset().unwrap().starts_with(&expected_message));
}

// TODO: fix
#[test_connector]
fn dev_diagnostic_returns_create_migration_when_the_database_is_behind(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    assert!(action.is_create_migration());
}

#[test_connector]
fn dev_diagnostic_can_detect_when_the_migrations_directory_is_behind(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let name = api
        .create_migration("second-migration", &dm2, &directory)
        .send_sync()
        .into_output()
        .generated_migration_name;

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let second_migration_folder_path = directory.path().join(&name);
    std::fs::remove_dir_all(second_migration_folder_path).unwrap();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    let message = action.as_reset().unwrap();
    assert!(message.contains("- Drift detected: Your database schema is not in sync with your migration history"));
    assert!(message.contains(&format!(
        "The following migration(s) are applied to the database but missing from the local migrations directory: {name}"
    )));
}

#[test_connector]
fn dev_diagnostic_can_detect_when_history_diverges(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let first_migration_name = api
        .create_migration("1-initial", &dm1, &directory)
        .send_sync()
        .into_output()
        .generated_migration_name;

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let deleted_migration_name = api
        .create_migration("2-second-migration", &dm2, &directory)
        .send_sync()
        .into_output()
        .generated_migration_name;

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["1-initial", "2-second-migration"]);

    let second_migration_folder_path = directory.path().join(&deleted_migration_name);
    std::fs::remove_dir_all(second_migration_folder_path).unwrap();

    let dm3 = api.datamodel_with_provider(
        r#"
        model Dog {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("3-create-dog", &dm3, &directory)
        .draft(true)
        .send_sync()
        .assert_migration_directories_count(2);

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    let message = action.as_reset().unwrap();

    assert!(message.contains("Drift detected: Your database schema is not in sync with your migration history"));
    assert!(message.contains(&format!("- The migrations recorded in the database diverge from the local migrations directory. Last common migration: `{first_migration_name}`. Migrations applied to the database but absent from the migrations directory are: {deleted_migration_name}")));
}

// TODO: fix
#[test_connector]
fn dev_diagnostic_can_detect_edited_migrations(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let (initial_migration_name, initial_path) = {
        let out = api.create_migration("initial", &dm1, &directory).send_sync();
        let path = out.migration_script_path();
        (out.into_output().generated_migration_name, path)
    };

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let mut file = std::fs::OpenOptions::new().append(true).open(initial_path).unwrap();
    file.write_all(b"-- test\nSELECT 1;").unwrap();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    let expected_message = format!("The migration `{initial_migration_name}` was modified after it was applied.");

    assert_eq!(action.as_reset(), Some(expected_message.as_str()));
}

#[test_connector]
fn dev_diagnostic_reports_migrations_failing_to_apply_cleanly(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let (initial_migration_name, initial_path) = {
        let out = api.create_migration("initial", &dm1, &directory).send_sync();
        let path = out.migration_script_path();
        (out.into_output().generated_migration_name, path)
    };

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let mut file = std::fs::OpenOptions::new().append(true).open(initial_path).unwrap();
    file.write_all(b"SELECT YOLO;\n").unwrap();

    let err = api.dev_diagnostic(&directory).send_unwrap_err().to_user_facing();

    let known_err = err.as_known().unwrap();

    assert_eq!(known_err.error_code, MigrationDoesNotApplyCleanly::ERROR_CODE);
    assert!(known_err.message.contains(initial_migration_name.as_str()));
}

#[test_connector]
fn dev_diagnostic_with_a_nonexistent_migrations_directory_works(api: TestApi) {
    let directory = api.create_migrations_directory();

    std::fs::remove_dir(directory.path()).unwrap();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();
    assert!(action.is_create_migration());
}

#[test_connector]
fn with_a_failed_migration(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model catsu {
            id Int @id
        }
    "#,
    );

    let CreateMigrationOutput {
        generated_migration_name,
        ..
    } = api
        .create_migration("01-init", &dm, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(1)
        .modify_migration(|migration| {
            migration.clear();
            migration.push_str("CREATE_BROKEN");
        })
        .into_output();

    let err = api
        .apply_migrations(&migrations_directory)
        .send_unwrap_err()
        .to_string();

    if api.is_mssql() {
        assert!(err.contains("Could not find stored procedure"), "{}", err)
    } else {
        assert!(&err.contains("syntax"), "{}", err)
    }

    std::fs::remove_dir_all(migrations_directory.path().join(&generated_migration_name)).unwrap();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&migrations_directory).send().into_output();

    assert!(action
        .as_reset()
        .unwrap()
        .contains(&format!("The migration `{generated_migration_name}` failed.")));
}

#[test_connector]
fn with_an_invalid_unapplied_migration_should_report_it(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model catcat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let dm2 = api.datamodel_with_provider(
        r#"
        model catcat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let CreateMigrationOutput {
        generated_migration_name,
        ..
    } = api
        .create_migration("second-migration", &dm2, &directory)
        .send_sync()
        .modify_migration(|script| {
            *script = "CREATE BROKEN".into();
        })
        .into_output();

    let err = api
        .dev_diagnostic(&directory)
        .send_unwrap_err()
        .to_user_facing()
        .unwrap_known();

    let expected_msg =
        format!("Migration `{generated_migration_name}` failed to apply cleanly to the shadow database. \nError");

    assert_eq!(err.error_code, MigrationDoesNotApplyCleanly::ERROR_CODE);
    assert!(err.message.starts_with(&expected_msg));
}

#[test_connector(tags(Postgres))]
fn drift_can_be_detected_without_migrations_table_dev(api: TestApi) {
    let directory = api.create_migrations_directory();

    api.raw_cmd("CREATE TABLE \"cat\" (\nid SERIAL PRIMARY KEY\n);");

    let dm1 = r#"
        model cat {
            id      Int @id @default(autoincrement())
        }
    "#;

    api.create_migration("initial", dm1, &directory).send_sync();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    let expect = expect![[r#"
        Drift detected: Your database schema is not in sync with your migration history.

        The following is a summary of the differences between the expected database schema given your migrations files, and the actual schema of the database.

        It should be understood as the set of changes to get from the expected schema to the actual schema.

        If you are running this the first time on an existing database, please make sure to read this documentation page:
        https://www.prisma.io/docs/guides/database/developing-with-prisma-migrate/troubleshooting-development

        [+] Added tables
          - cat
    "#]];

    expect.assert_eq(action.as_reset().unwrap());
}

#[test_connector(tags(Postgres))]
fn drift_detect_first_time_message_should_not_be_dispyed_if_migration_table_exists(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model catcat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    api.raw_cmd("CREATE TABLE \"cat\" (\nid SERIAL PRIMARY KEY\n);");

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().into_output();

    let expect = expect![[r#"
        Drift detected: Your database schema is not in sync with your migration history.

        The following is a summary of the differences between the expected database schema given your migrations files, and the actual schema of the database.

        It should be understood as the set of changes to get from the expected schema to the actual schema.

        [+] Added tables
          - cat
    "#]];

    expect.assert_eq(action.as_reset().unwrap());
}

#[test_connector(tags(Mysql8), exclude(Vitess))]
fn dev_diagnostic_shadow_database_creation_error_is_special_cased_mysql(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#;

    api.create_migration("initial", dm1, &directory).send_sync();

    api.raw_cmd(&format!(
        "
            DROP USER IF EXISTS 'prismashadowdbtestuser';
            CREATE USER 'prismashadowdbtestuser' IDENTIFIED by '1234batman';
            GRANT ALL PRIVILEGES ON {}.* TO 'prismashadowdbtestuser';
            ",
        api.connection_info().dbname().unwrap(),
    ));

    let db_url: url::Url = api.connection_string().parse().unwrap();

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "mysql"
            url = "mysql://prismashadowdbtestuser:1234batman@{dbhost}:{dbport}/{dbname}"
        }}
        "#,
        dbhost = db_url.host().unwrap(),
        dbname = api.connection_info().dbname().unwrap(),
        dbport = db_url.port().unwrap_or(3306),
    );

    let migrations_list = list_migrations(&directory.keep()).unwrap();

    let err = tok(async {
        let migration_api = schema_api(Some(datamodel), None).unwrap();
        migration_api
            .dev_diagnostic(DevDiagnosticInput {
                migrations_list,
                schema_filter: None,
            })
            .await
    })
    .unwrap_err()
    .to_user_facing()
    .unwrap_known();

    assert!(err.message.starts_with("Prisma Migrate could not create the shadow database. Please make sure the database user has permission to create databases. Read more about the shadow database (and workarounds) at https://pris.ly/d/migrate-shadow"), "{err:?}");
}

#[test_connector(tags(Postgres12))]
fn dev_diagnostic_shadow_database_creation_error_is_special_cased_postgres(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#;

    api.create_migration("initial", dm1, &directory).send_sync();

    api.raw_cmd(
        "
            DROP USER IF EXISTS prismashadowdbtestuser;
            CREATE USER prismashadowdbtestuser PASSWORD '1234batman' LOGIN;
            ",
    );

    let db_url: url::Url = api.connection_string().parse().unwrap();

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "postgresql"
            url = "postgresql://prismashadowdbtestuser:1234batman@{dbhost}:{dbport}/{dbname}"
        }}
        "#,
        dbhost = db_url.host().unwrap(),
        dbname = api.connection_info().dbname().unwrap(),
        dbport = db_url.port().unwrap(),
    );

    let migrations_list = list_migrations(&directory.keep()).unwrap();

    let err = tok(async move {
        let migration_api = schema_api(Some(datamodel), None).unwrap();
        migration_api
            .dev_diagnostic(DevDiagnosticInput {
                migrations_list,
                schema_filter: None,
            })
            .await
    })
    .unwrap_err()
    .to_user_facing()
    .unwrap_known();

    assert!(err.message.starts_with("Prisma Migrate could not create the shadow database. Please make sure the database user has permission to create databases. Read more about the shadow database (and workarounds) at https://pris.ly/d/migrate-shadow"));
}

// (Hopefully) Temporarily commented out because this test is flaky in CI.
// #[test_connector(tags("mssql"))]
// fn dev_diagnostic_shadow_database_creation_error_is_special_cased_mssql(api: TestApi)  {
//     let directory = api.create_migrations_directory();

//     let dm1 = r#"
//         model Cat {
//             id      Int @id @default(autoincrement())
//         }
//     "#;

//     api.create_migration("initial", dm1, &directory).send();

//     api.database()
//         .raw_cmd(
//             "
//             CREATE LOGIN prismashadowdbtestuser2
//                 WITH PASSWORD = '1234batmanZ';

//             CREATE USER prismashadowdbuser2 FOR LOGIN prismashadowdbtestuser2;

//             GRANT SELECT TO prismashadowdbuser2;
//             ",
//         )
//         .await
//         .ok();

//     let (host, port) = test_setup::db_host_and_port_mssql_2019();

//     let datamodel = format!(
//         r#"
//         datasource db {{
//             provider = "sqlserver"
//             url = "sqlserver://{dbhost}:{dbport};database={dbname};user=prismashadowdbtestuser2;password=1234batmanZ;trustservercertificate=true"
//         }}
//         "#,
//         dbhost = host,
//         dbname = api.connection_info().dbname().unwrap(),
//         dbport = port,
//     );

//     let mut tries = 0;

//     let migration_api = loop {
//         if tries > 5 {
//             panic!("Failed to connect to mssql more than five times.");
//         }

//         let result = migration_api(&datamodel).await;

//         match result {
//             Ok(api) => break api,
//             Err(err) => {
//                 tries += 1;
//                 eprintln!("got err, sleeping\nerr:{:?}", err);
//                 tokio::time::sleep(std::time::Duration::from_millis(200)).await;
//             }
//         }
//     };

//     let err = migration_api
//         .dev_diagnostic(&DevDiagnosticInput {
//             migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
//         })
//         .await
//         .unwrap_err()
//         .to_user_facing()
//         .unwrap_known();

//     assert_eq!(err.error_code, ShadowDbCreationError::ERROR_CODE);
//     assert!(err.message.starts_with("Prisma Migrate could not create the shadow database. Please make sure the database user has permission to create databases. Read more at https://pris.ly/d/migrate-shadow"));

//
// }

#[test]
fn dev_diagnostic_multi_schema_does_not_panic() {
    let db = test_setup::only!(Postgres);
    let (_, url) = tok(test_setup::postgres::create_postgres_database(
        db.url(),
        "dev_diagnostic_multi_schema",
    ))
    .unwrap();

    let provider = test_setup::TestApiArgs::new("dev_diagnostic_multi_schema_does_not_panic", &[], &[])
        .provider()
        .to_owned();

    let schema = format! {r#"
        datasource db {{
            provider = "{provider}"
            url = "{url}"
            schemas = ["prisma-tests", "auth"]
        }}

        generator js {{
            provider = "prisma-client-js"
            previewFeatures = ["multiSchema"]
        }}

        model users {{
          id       String    @id @db.Uuid
          profiles profiles?

          @@schema("auth")
        }}

        model profiles {{
          id    String @id @db.Uuid
          users users  @relation(fields: [id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("prisma-tests")
        }}
    "#};

    let setup = r#"
-- ./sql/ddl.sql

CREATE SCHEMA auth;

-- auth.users definition
CREATE TABLE auth.users (
    id uuid NOT NULL,
    CONSTRAINT users_pkey PRIMARY KEY (id)
);

-- "prisma-tests".profiles definition
CREATE TABLE "prisma-tests".profiles (
    id uuid NOT NULL,
    CONSTRAINT profiles_pkey PRIMARY KEY (id)
);

-- "prisma-tests".profiles foreign keys
ALTER TABLE "prisma-tests".profiles ADD CONSTRAINT profiles_id_fkey FOREIGN KEY (id) REFERENCES auth.users(id);
    "#;

    let tempdir = tempfile::tempdir().unwrap();
    std::fs::write(tempdir.path().join("schema.prisma"), &schema).unwrap();

    let api = schema_core::schema_api(Some(schema), None).unwrap();

    tok(api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer { url }),
        script: setup.to_owned(),
    }))
    .unwrap();

    let migrations_list = list_migrations(&tempdir.keep()).unwrap();

    tok(api.dev_diagnostic(DevDiagnosticInput {
        migrations_list,
        schema_filter: None,
    }))
    .unwrap();
}
