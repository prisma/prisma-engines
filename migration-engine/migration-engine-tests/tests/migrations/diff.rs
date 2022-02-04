use migration_core::{
    json_rpc::types::{DiffTarget, PathContainer},
    migration_connector::MigrationConnector,
};
use migration_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use std::sync::Arc;

#[test_connector(tags(Sqlite))]
fn diffing_postgres_schemas_when_initialized_on_sqlite(mut api: TestApi) {
    // We should get a postgres diff.

    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());

    api.connector.set_host(host.clone());

    let from = r#"
        datasource db {
            provider = "postgresql"
            url = "postgresql://example.com/test"
        }

        model TestModel {
            id Int @id @default(autoincrement())
            names String
        }
    "#;

    let from_file = write_file_to_tmp(from, &tempdir, "from");

    let to = r#"
        datasource db {
            provider = "postgresql"
            url = "postgresql://example.com/test"
        }

        model TestModel {
            id Int @id @default(autoincrement())
            names String[]
        }

        model TestModel2 {
            id Int @id @default(autoincrement())
        }
    "#;

    let to_file = write_file_to_tmp(to, &tempdir, "to");

    api.diff(DiffParams {
        from: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: from_file.to_string_lossy().into_owned(),
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: to_file.to_string_lossy().into_owned(),
        }),
        script: true,
    })
    .unwrap();

    api.diff(DiffParams {
        from: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: from_file.to_string_lossy().into_owned(),
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: to_file.to_string_lossy().into_owned(),
        }),
        script: false,
    })
    .unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- AlterTable\nALTER TABLE \"TestModel\" DROP COLUMN \"names\",\nADD COLUMN     \"names\" TEXT[];\n\n-- CreateTable\nCREATE TABLE \"TestModel2\" (\n    \"id\" SERIAL NOT NULL,\n\n    CONSTRAINT \"TestModel2_pkey\" PRIMARY KEY (\"id\")\n);\n",
            "\n[+] Added tables\n  - TestModel2\n\n[*] Changed the `TestModel` table\n  [*] Column `names` would be dropped and recreated(changed from Required to List, type changed)\n",
        ]
    "#]];

    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector(tags(Postgres))]
fn from_empty_to_migrations_directory(mut api: TestApi) {
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());
    let base_dir = tempfile::TempDir::new().unwrap();
    let first_migration_directory_path = base_dir.path().join("01firstmigration");
    let first_migration_file_path = first_migration_directory_path.join("migration.sql");
    let migrations_lock_path = base_dir.path().join("migration_lock.toml");
    std::fs::write(
        &migrations_lock_path,
        &format!("provider = \"{}\"", api.args().provider()),
    )
    .unwrap();
    std::fs::create_dir_all(&first_migration_directory_path).unwrap();
    std::fs::write(
        &first_migration_file_path,
        "CREATE TABLE cats ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT false );",
    )
    .unwrap();

    let params = DiffParams {
        from: DiffTarget::Empty,
        to: DiffTarget::Migrations(PathContainer {
            path: base_dir.path().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: Some(api.connection_string().to_owned()),
    };

    api.diff(params).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- CreateTable\nCREATE TABLE \"cats\" (\n    \"id\" INTEGER NOT NULL,\n    \"moos\" BOOLEAN DEFAULT false,\n\n    CONSTRAINT \"cats_pkey\" PRIMARY KEY (\"id\")\n);\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

// TODO: test migration directories without migrations lock

#[test_connector(exclude(Sqlite))] // no shadow database url on sqlite
fn from_empty_to_migrations_folder_without_shadow_db_url_must_error(mut api: TestApi) {
    let base_dir = tempfile::TempDir::new().unwrap();
    let first_migration_directory_path = base_dir.path().join("01firstmigration");
    let first_migration_file_path = first_migration_directory_path.join("migration.sql");
    let migrations_lock_path = base_dir.path().join("migration_lock.toml");
    std::fs::write(
        &migrations_lock_path,
        &format!("provider = \"{}\"", api.args().provider()),
    )
    .unwrap();
    std::fs::create_dir_all(&first_migration_directory_path).unwrap();
    std::fs::write(
        &first_migration_file_path,
        "CREATE TABLE cats ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT false );",
    )
    .unwrap();

    let params = DiffParams {
        from: DiffTarget::Empty,
        to: DiffTarget::Migrations(PathContainer {
            path: base_dir.path().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None, // TODO: ?
    };

    let err = api.diff(params).unwrap_err();

    let expected_error = expect![[r#"
        You must pass the --shadow-database-url if you want to diff a migrations directory.
    "#]];
    expected_error.assert_eq(&err.to_string());
}

#[test_connector]
fn from_schema_datamodel_to_url(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_schema = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model cows {
            id Int @id @default(autoincrement())
            moos Boolean
        }
    "#;
    let schema_path = write_file_to_tmp(&first_schema, &tempdir, "schema.prisma");
    let second_url = format!("file:{}/second_db.sqlite", base_dir_str);

    api.block_on(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cats ( id INTEGER PRIMARY KEY, meows BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    let input = DiffParams {
        from: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: schema_path.to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
    };

    api.diff(input).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"cows\";\nPRAGMA foreign_keys=on;\n\n-- CreateTable\nCREATE TABLE \"cats\" (\n    \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,\n    \"meows\" BOOLEAN DEFAULT true\n);\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector]
fn from_schema_datasource_to_url(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{}/first_db.sqlite", base_dir_str);
    let second_url = format!("file:{}/second_db.sqlite", base_dir_str);

    api.block_on(async {
        let q = quaint::single::Quaint::new(&first_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cows ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    api.block_on(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cats ( id INTEGER PRIMARY KEY, meows BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    let schema_content = format!(
        r#"
          datasource db {{
              provider = "sqlite"
              url = "{}"
          }}
        "#,
        first_url.replace('\\', "\\\\")
    );
    let schema_path = write_file_to_tmp(&schema_content, &tempdir, "schema.prisma");

    let input = DiffParams {
        from: DiffTarget::SchemaDatasource(SchemaContainer {
            schema: schema_path.to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
    };

    api.diff(input).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"cows\";\nPRAGMA foreign_keys=on;\n\n-- CreateTable\nCREATE TABLE \"cats\" (\n    \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,\n    \"meows\" BOOLEAN DEFAULT true\n);\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector]
fn from_url_to_url(mut api: TestApi) {
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{}/first_db.sqlite", base_dir_str);
    let second_url = format!("file:{}/second_db.sqlite", base_dir_str);

    api.block_on(async {
        let q = quaint::single::Quaint::new(&first_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cows ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    api.block_on(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cats ( id INTEGER PRIMARY KEY, meows BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    let input = DiffParams {
        from: DiffTarget::Url(UrlContainer { url: first_url }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
    };

    api.diff(input).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"cows\";\nPRAGMA foreign_keys=on;\n\n-- CreateTable\nCREATE TABLE \"cats\" (\n    \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,\n    \"meows\" BOOLEAN DEFAULT true\n);\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test]
fn diffing_mongo_schemas_to_script_returns_a_nice_error() {
    let tempdir = tempfile::tempdir().unwrap();

    let from = r#"
        datasource db {
            provider = "mongodb"
            url = "mongo+srv://test"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["mongodb"]
        }

        model TestModel {
            id String @id @default(auto()) @map("_id") @db.ObjectId
            names String
        }
    "#;

    let from_file = write_file_to_tmp(from, &tempdir, "from");

    let to = r#"
        datasource db {
            provider = "mongodb"
            url = "mongo+srv://test"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["mongodb"]
        }


        model TestModel {
            id String @id @default(auto()) @map("_id") @db.ObjectId
            names String[]

            @@index([names])
        }

        model TestModel2 {
            id String @id @default(auto()) @map("_id") @db.ObjectId
        }
    "#;

    let to_file = write_file_to_tmp(to, &tempdir, "to");

    let params = DiffParams {
        from: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: from_file.to_string_lossy().into_owned(),
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: to_file.to_string_lossy().into_owned(),
        }),
        script: true,
    };

    let expected = expect![[r#"
        Rendering to a script is not supported on MongoDB.
    "#]];
    expected.assert_eq(&diff_error(params));
}

#[test]
fn diffing_mongo_schemas_works() {
    let tempdir = tempfile::tempdir().unwrap();

    let from = r#"
        datasource db {
            provider = "mongodb"
            url = "mongo+srv://test"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["mongodb"]
        }

        model TestModel {
            id String @id @default(auto()) @map("_id") @db.ObjectId
            names String
        }
    "#;

    let from_file = write_file_to_tmp(from, &tempdir, "from");

    let to = r#"
        datasource db {
            provider = "mongodb"
            url = "mongo+srv://test"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["mongodb"]
        }


        model TestModel {
            id String @id @default(auto()) @map("_id") @db.ObjectId
            names String[]

            @@index([names])
        }

        model TestModel2 {
            id String @id @default(auto()) @map("_id") @db.ObjectId
        }
    "#;

    let to_file = write_file_to_tmp(to, &tempdir, "to");

    let params = DiffParams {
        from: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: from_file.to_string_lossy().into_owned(),
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: to_file.to_string_lossy().into_owned(),
        }),
        script: false,
    };

    let expected_printed_messages = expect![[r#"
        [+] Collection `TestModel2`
        [+] Index `TestModel_names_idx` on ({"names":1})
    "#]];

    expected_printed_messages.assert_eq(&diff_output(params));
}

#[test]
fn with_missing_prisma_schema_should_return_helpful_error() {
    // We are counting on this path not existing.
    let tmp_path = std::env::temp_dir().join("prisma_migrate_diff_test_this_file_does_not_exist");
    let tmp_path_str = tmp_path.to_str().unwrap();

    // We want to test for both --schema-datamodel and --schema-datasource
    let test_with_from_target = |from_target: DiffTarget| {
        let params = DiffParams {
            from: from_target,
            script: false,
            shadow_database_url: None,
            to: DiffTarget::Empty,
        };

        let error = diff_error(params);
        assert!(error.match_indices(tmp_path_str).next().is_some());

        let expected = if cfg!(windows) {
            expect![[r#"
                Error trying to read Prisma schema file at `<the-path>`.
                The system cannot find the file specified. (os error 2)
            "#]]
        } else {
            expect![[r#"
                Error trying to read Prisma schema file at `<the-path>`.
                No such file or directory (os error 2)
            "#]]
        };

        expected.assert_eq(&error.replace(tmp_path_str, "<the-path>"));
    };

    test_with_from_target(DiffTarget::SchemaDatamodel(SchemaContainer {
        schema: tmp_path_str.to_owned(),
    }));
    test_with_from_target(DiffTarget::SchemaDatasource(SchemaContainer {
        schema: tmp_path_str.to_owned(),
    }));
}

// Call diff, and expect it to error. Return the error.
fn diff_error(params: DiffParams) -> String {
    let api = migration_core::migration_api(None, None).unwrap();
    let result = test_setup::runtime::run_with_tokio(api.diff(params));
    result.unwrap_err().to_string()
}

// Call diff, and expect it to succeed. Return what would be printed to stdout.
fn diff_output(params: DiffParams) -> String {
    let host = Arc::new(TestConnectorHost::default());
    let api = migration_core::migration_api(None, Some(host.clone())).unwrap();
    test_setup::runtime::run_with_tokio(api.diff(params)).unwrap();
    let printed_messages = host.printed_messages.lock().unwrap();
    assert!(printed_messages.len() == 1, "{:?}", printed_messages);
    printed_messages[0].clone()
}

fn write_file_to_tmp(contents: &str, tempdir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let tempfile_path = tempdir.path().join(name);
    std::fs::write(&tempfile_path, contents.as_bytes()).unwrap();
    tempfile_path
}
