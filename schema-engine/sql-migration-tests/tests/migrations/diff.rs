use psl::parser_database::NoExtensionTypes;
use quaint::{prelude::Queryable, single::Quaint};
use schema_core::{
    commands::diff_cli,
    json_rpc::types::{DiffTarget, SchemaFilter, SchemasContainer, SchemasWithConfigDir},
    schema_connector::SchemaConnector,
};
use sql_migration_tests::{
    test_api::*,
    utils::{list_migrations, to_schema_containers},
};
use std::sync::Arc;

#[test_connector(tags(Sqlite, Mysql, Postgres, CockroachDb, Mssql))]
fn from_unique_index_to_without(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());

    api.connector.set_host(host.clone());

    let from_schema = api.datamodel_with_provider(
        r#"
            model Post {
                id       Int    @id
                title    String
                author   User?  @relation(fields: [authorId], references: [id])
                authorId Int?   @unique
                //              ^^^^^^^ this will be removed later
            }

            model User {
                id    Int     @id
                name  String?
                posts Post[]
            }
        "#,
    );

    let to_schema = api.datamodel_with_provider(
        r#"
            model Post {
                id       Int    @id
                title    String
                author   User?  @relation(fields: [authorId], references: [id])
                authorId Int?
            }

            model User {
                id    Int     @id
                name  String?
                posts Post[]
            }
        "#,
    );

    let from_file = write_file_to_tmp(&from_schema, &tempdir, "from");
    let to_file = write_file_to_tmp(&to_schema, &tempdir, "to");

    api.diff(DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: from_file.to_string_lossy().into_owned(),
                content: from_schema.to_string(),
            }],
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: to_file.to_string_lossy().into_owned(),
                content: to_schema.to_string(),
            }],
        }),
        script: true,
        filters: SchemaFilter::default(),
    })
    .unwrap();

    let expected_printed_messages = if api.is_vitess() {
        expect![[r#"
            [
                "-- DropIndex\nDROP INDEX `Post_authorId_key` ON `Post`;\n",
            ]
            "#]]
    } else if api.is_mysql() {
        // MySQL requires dropping the foreign key before dropping the index.
        expect![[r#"
            [
                "-- DropForeignKey\nALTER TABLE `Post` DROP FOREIGN KEY `Post_authorId_fkey`;\n\n-- DropIndex\nDROP INDEX `Post_authorId_key` ON `Post`;\n\n-- AddForeignKey\nALTER TABLE `Post` ADD CONSTRAINT `Post_authorId_fkey` FOREIGN KEY (`authorId`) REFERENCES `User`(`id`) ON DELETE SET NULL ON UPDATE CASCADE;\n",
            ]
        "#]]
    } else if api.is_sqlite() {
        expect![[r#"
            [
                "-- DropIndex\nDROP INDEX \"Post_authorId_key\";\n",
            ]
        "#]]
    } else if api.is_postgres() || api.is_cockroach() {
        expect![[r#"
            [
                "-- DropIndex\nDROP INDEX \"public\".\"Post_authorId_key\";\n",
            ]
        "#]]
    } else if api.is_mssql() {
        expect![[r#"
            [
                "BEGIN TRY\n\nBEGIN TRAN;\n\n-- DropIndex\nDROP INDEX [Post_authorId_key] ON [dbo].[Post];\n\nCOMMIT TRAN;\n\nEND TRY\nBEGIN CATCH\n\nIF @@TRANCOUNT > 0\nBEGIN\n    ROLLBACK TRAN;\nEND;\nTHROW\n\nEND CATCH\n",
            ]
        "#]]
    } else {
        unreachable!()
    };

    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector]
fn from_unique_index_to_pk(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());

    api.connector.set_host(host.clone());

    let from_schema = api.datamodel_with_provider(
        r#"
            model A {
                id   Int     @unique
                name String?
            }

            model B {
                x Int
                y Int

                @@unique([x, y])
            }

            model C {
                id        Int @id
                secondary Int @unique
            }
        "#,
    );

    let to_schema = api.datamodel_with_provider(
        r#"
            model A {
                id Int @id
            }

            model B {
                x Int
                y Int

                @@id([x, y])
            }

            // Dropping the unique index on `secondary` will not be reordered
            // so it will appear before the modifications on A and B in the migration.
            model C {
                id        Int @id
                secondary Int
            }
        "#,
    );

    let from_file = write_file_to_tmp(&from_schema, &tempdir, "from");
    let to_file = write_file_to_tmp(&to_schema, &tempdir, "to");

    api.diff(DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: from_file.to_string_lossy().into_owned(),
                content: from_schema.to_string(),
            }],
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: to_file.to_string_lossy().into_owned(),
                content: to_schema.to_string(),
            }],
        }),
        script: true,
        filters: SchemaFilter::default(),
    })
    .unwrap();

    let expected_printed_messages = if api.is_mysql() {
        expect![[r#"
            [
                [
                    "-- DropIndex",
                    "DROP INDEX `C_secondary_key` ON `C`;",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE `A` DROP COLUMN `name`,",
                    "    ADD PRIMARY KEY (`id`);",
                    "",
                    "-- DropIndex",
                    "DROP INDEX `A_id_key` ON `A`;",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE `B` ADD PRIMARY KEY (`x`, `y`);",
                    "",
                    "-- DropIndex",
                    "DROP INDEX `B_x_y_key` ON `B`;",
                    "",
                ],
            ]
        "#]]
    } else if api.is_postgres() && !api.is_cockroach() {
        expect![[r#"
            [
                [
                    "-- DropIndex",
                    "DROP INDEX \"public\".\"C_secondary_key\";",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE \"public\".\"A\" DROP COLUMN \"name\",",
                    "ADD CONSTRAINT \"A_pkey\" PRIMARY KEY (\"id\");",
                    "",
                    "-- DropIndex",
                    "DROP INDEX \"public\".\"A_id_key\";",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE \"public\".\"B\" ADD CONSTRAINT \"B_pkey\" PRIMARY KEY (\"x\", \"y\");",
                    "",
                    "-- DropIndex",
                    "DROP INDEX \"public\".\"B_x_y_key\";",
                    "",
                ],
            ]
        "#]]
    } else if api.is_cockroach() {
        expect![[r#"
            [
                [
                    "-- DropIndex",
                    "DROP INDEX \"C_secondary_key\";",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE \"A\" DROP COLUMN \"name\";",
                    "ALTER TABLE \"A\" ADD CONSTRAINT \"A_pkey\" PRIMARY KEY (\"id\");",
                    "",
                    "-- DropIndex",
                    "DROP INDEX \"A_id_key\";",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE \"B\" ADD CONSTRAINT \"B_pkey\" PRIMARY KEY (\"x\", \"y\");",
                    "",
                    "-- DropIndex",
                    "DROP INDEX \"B_x_y_key\";",
                    "",
                ],
            ]
        "#]]
    } else if api.is_mssql() {
        expect![[r#"
            [
                [
                    "BEGIN TRY",
                    "",
                    "BEGIN TRAN;",
                    "",
                    "-- DropIndex",
                    "DROP INDEX [C_secondary_key] ON [dbo].[C];",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE [dbo].[A] DROP COLUMN [name];",
                    "ALTER TABLE [dbo].[A] ADD CONSTRAINT A_pkey PRIMARY KEY CLUSTERED ([id]);",
                    "",
                    "-- DropIndex",
                    "DROP INDEX [A_id_key] ON [dbo].[A];",
                    "",
                    "-- AlterTable",
                    "ALTER TABLE [dbo].[B] ADD CONSTRAINT B_pkey PRIMARY KEY CLUSTERED ([x],[y]);",
                    "",
                    "-- DropIndex",
                    "DROP INDEX [B_x_y_key] ON [dbo].[B];",
                    "",
                    "COMMIT TRAN;",
                    "",
                    "END TRY",
                    "BEGIN CATCH",
                    "",
                    "IF @@TRANCOUNT > 0",
                    "BEGIN",
                    "    ROLLBACK TRAN;",
                    "END;",
                    "THROW",
                    "",
                    "END CATCH",
                    "",
                ],
            ]
        "#]]
    } else if api.is_sqlite() {
        expect![[r#"
            [
                [
                    "-- DropIndex",
                    "DROP INDEX \"C_secondary_key\";",
                    "",
                    "-- RedefineTables",
                    "PRAGMA defer_foreign_keys=ON;",
                    "PRAGMA foreign_keys=OFF;",
                    "CREATE TABLE \"new_A\" (",
                    "    \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT",
                    ");",
                    "INSERT INTO \"new_A\" (\"id\") SELECT \"id\" FROM \"A\";",
                    "DROP TABLE \"A\";",
                    "ALTER TABLE \"new_A\" RENAME TO \"A\";",
                    "CREATE TABLE \"new_B\" (",
                    "    \"x\" INTEGER NOT NULL,",
                    "    \"y\" INTEGER NOT NULL,",
                    "",
                    "    PRIMARY KEY (\"x\", \"y\")",
                    ");",
                    "INSERT INTO \"new_B\" (\"x\", \"y\") SELECT \"x\", \"y\" FROM \"B\";",
                    "DROP TABLE \"B\";",
                    "ALTER TABLE \"new_B\" RENAME TO \"B\";",
                    "PRAGMA foreign_keys=ON;",
                    "PRAGMA defer_foreign_keys=OFF;",
                    "",
                ],
            ]
        "#]]
    } else {
        unreachable!()
    };

    expected_printed_messages.assert_debug_eq(
        &host
            .printed_messages
            .lock()
            .unwrap()
            .iter()
            .map(|msg| msg.split('\n').map(<_>::to_owned).collect())
            .collect::<Vec<Vec<_>>>(),
    );
}

#[test_connector(tags(Sqlite))]
fn diffing_postgres_schemas_when_initialized_on_sqlite(mut api: TestApi) {
    // We should get a postgres diff.

    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());

    api.connector.set_host(host.clone());

    let from_schema = r#"
        datasource db {
            provider = "postgresql"
            url = "postgresql://example.com/test"
        }

        model TestModel {
            id Int @id @default(autoincrement())
            names String
        }
    "#;

    let from_file = write_file_to_tmp(from_schema, &tempdir, "from");

    let to_schema = r#"
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

    let to_file = write_file_to_tmp(to_schema, &tempdir, "to");

    api.diff(DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: from_file.to_string_lossy().into_owned(),
                content: from_schema.to_string(),
            }],
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: to_file.to_string_lossy().into_owned(),
                content: to_schema.to_string(),
            }],
        }),
        script: true,
        filters: SchemaFilter::default(),
    })
    .unwrap();

    api.diff(DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: from_file.to_string_lossy().into_owned(),
                content: from_schema.to_string(),
            }],
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: to_file.to_string_lossy().into_owned(),
                content: to_schema.to_string(),
            }],
        }),
        script: false,
        filters: SchemaFilter::default(),
    })
    .unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- AlterTable\nALTER TABLE \"public\".\"TestModel\" DROP COLUMN \"names\",\nADD COLUMN     \"names\" TEXT[];\n\n-- CreateTable\nCREATE TABLE \"public\".\"TestModel2\" (\n    \"id\" SERIAL NOT NULL,\n\n    CONSTRAINT \"TestModel2_pkey\" PRIMARY KEY (\"id\")\n);\n",
            "\n[+] Added tables\n  - TestModel2\n\n[*] Changed the `TestModel` table\n  [*] Column `names` would be dropped and recreated (changed from Required to List, type changed)\n",
        ]
    "#]];

    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn from_empty_to_migrations_directory(mut api: TestApi) {
    let base_dir = tempfile::TempDir::new().unwrap();
    let first_migration_directory_path = base_dir.path().join("01firstmigration");
    let first_migration_file_path = first_migration_directory_path.join("migration.sql");
    let migrations_lock_path = base_dir.path().join("migration_lock.toml");
    std::fs::write(
        migrations_lock_path,
        format!("provider = \"{}\"", api.args().provider()),
    )
    .unwrap();
    std::fs::create_dir_all(&first_migration_directory_path).unwrap();
    std::fs::write(
        first_migration_file_path,
        "CREATE TABLE cats ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT false );",
    )
    .unwrap();

    let migrations_list = list_migrations(&base_dir.keep()).unwrap();

    let params = DiffParams {
        exit_code: None,
        from: DiffTarget::Empty,
        to: DiffTarget::Migrations(migrations_list),
        script: true,
        shadow_database_url: Some(api.connection_string().to_owned()),
        filters: SchemaFilter::default(),
    };

    let host = Arc::new(TestConnectorHost::default());
    tok(diff_cli(params, host.clone(), &NoExtensionTypes)).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- CreateSchema\nCREATE SCHEMA IF NOT EXISTS \"public\";\n\n-- CreateTable\nCREATE TABLE \"public\".\"cats\" (\n    \"id\" INTEGER NOT NULL,\n    \"moos\" BOOLEAN DEFAULT false,\n\n    CONSTRAINT \"cats_pkey\" PRIMARY KEY (\"id\")\n);\n",
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
        migrations_lock_path,
        format!("provider = \"{}\"", api.args().provider()),
    )
    .unwrap();
    std::fs::create_dir_all(&first_migration_directory_path).unwrap();
    std::fs::write(
        first_migration_file_path,
        "CREATE TABLE cats ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT false );",
    )
    .unwrap();

    let migrations_list = list_migrations(&base_dir.keep()).unwrap();

    let params = DiffParams {
        exit_code: None,
        from: DiffTarget::Empty,
        to: DiffTarget::Migrations(migrations_list),
        script: true,
        shadow_database_url: None, // TODO: ?
        filters: SchemaFilter::default(),
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
    let schema_path = write_file_to_tmp(first_schema, &tempdir, "schema.prisma");
    let second_url = format!("file:{base_dir_str}/second_db.sqlite");

    tok(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cats ( id INTEGER PRIMARY KEY, meows BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    let input = DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: first_schema.to_string(),
            }],
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
        filters: SchemaFilter::default(),
    };

    api.diff(input).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"cows\";\nPRAGMA foreign_keys=on;\n\n-- CreateTable\nCREATE TABLE \"cats\" (\n    \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,\n    \"meows\" BOOLEAN DEFAULT true\n);\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector(tags(Sqlite))]
fn from_schema_datasource_relative(mut api: TestApi) {
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let tmpdir = tempfile::tempdir().unwrap();
    let prisma_dir = tmpdir.path().join("prisma");
    std::fs::create_dir_all(&prisma_dir).unwrap();
    let schema_path = prisma_dir.join("schema.prisma");
    let schema = r#"
        datasource db {
          provider = "sqlite"
          url = "file:./dev.db"
        }
    "#;

    std::fs::write(&schema_path, schema).unwrap();

    let expected_sqlite_path = prisma_dir.join("dev.db");

    tok(async {
        let path = expected_sqlite_path.to_str().unwrap();
        let quaint = Quaint::new(&format!("file:{path}")).await.unwrap();
        quaint.raw_cmd("CREATE TABLE foo (id INT PRIMARY KEY)").await.unwrap();
    });

    assert!(expected_sqlite_path.exists());

    let params = DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatasource(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: schema.to_string(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Empty,
        filters: SchemaFilter::default(),
    };

    api.diff(params).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"foo\";\nPRAGMA foreign_keys=on;\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector(tags(Sqlite))]
fn from_schema_datasource_to_url(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{base_dir_str}/first_db.sqlite");
    let second_url = format!("file:{base_dir_str}/second_db.sqlite");

    tok(async {
        let q = quaint::single::Quaint::new(&first_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cows ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    tok(async {
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
        exit_code: None,
        from: DiffTarget::SchemaDatasource(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: schema_content.to_string(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
        filters: SchemaFilter::default(),
    };

    api.diff(input).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"cows\";\nPRAGMA foreign_keys=on;\n\n-- CreateTable\nCREATE TABLE \"cats\" (\n    \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,\n    \"meows\" BOOLEAN DEFAULT true\n);\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector(tags(Sqlite))]
fn with_schema_filters(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{base_dir_str}/first_db.sqlite");
    let second_url = format!("file:{base_dir_str}/second_db.sqlite");

    tok(async {
        let q = quaint::single::Quaint::new(&first_url).await.unwrap();
        q.raw_cmd("CREATE TABLE external_table ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    // Ensure DB exists
    tok(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("SELECT 1;").await.unwrap();
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
        exit_code: None,
        from: DiffTarget::SchemaDatasource(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: schema_content.to_string(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
        filters: SchemaFilter {
            external_tables: vec!["external_table".to_string()],
            external_enums: vec![],
        },
    };

    api.diff(input).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- This is an empty migration.\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

#[test_connector(tags(Sqlite))]
fn with_invalid_schema_filter_sqlite(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{base_dir_str}/first_db.sqlite");
    let second_url = format!("file:{base_dir_str}/second_db.sqlite");

    // Ensure DB exists
    tok(async {
        let q = quaint::single::Quaint::new(&first_url).await.unwrap();
        q.raw_cmd("SELECT 1;").await.unwrap();
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
        exit_code: None,
        from: DiffTarget::SchemaDatasource(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: schema_content.to_string(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
        filters: SchemaFilter {
            external_tables: vec!["public.external_table".to_string()],
            external_enums: vec![],
        },
    };

    let err = api.diff(input).unwrap_err();

    assert_eq!(err.error_code(), Some("P3024"));
}

#[test_connector(tags(Postgres))]
fn with_invalid_schema_filter_postgres(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let connection_string = api.connection_string();

    let schema_content = format!(
        r#"
          datasource db {{
              provider = "postgresql"
              url = "{connection_string}"
          }}
        "#
    );
    let schema_path = write_file_to_tmp(&schema_content, &tempdir, "schema.prisma");

    let input = DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatasource(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: schema_content.to_string(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer {
            url: connection_string.to_string(),
        }),
        filters: SchemaFilter {
            external_tables: vec!["external_table".to_string()],
            external_enums: vec![],
        },
    };

    let err = api.diff(input).unwrap_err();

    assert_eq!(err.error_code(), Some("P3023"));
}

#[test_connector(tags(Sqlite))]
fn from_url_to_url(mut api: TestApi) {
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{base_dir_str}/first_db.sqlite");
    let second_url = format!("file:{base_dir_str}/second_db.sqlite");

    tok(async {
        let q = quaint::single::Quaint::new(&first_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cows ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    tok(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cats ( id INTEGER PRIMARY KEY, meows BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    let input = DiffParams {
        exit_code: None,
        from: DiffTarget::Url(UrlContainer { url: first_url }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
        filters: SchemaFilter::default(),
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
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: from_file.to_string_lossy().into_owned(),
                content: from.to_string(),
            }],
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: to_file.to_string_lossy().into_owned(),
                content: to.to_string(),
            }],
        }),
        script: true,
        filters: SchemaFilter::default(),
    };

    let expected = expect![[r#"
        Rendering to a script is not supported on MongoDB.
    "#]];
    expected.assert_eq(&diff_error(params));
}

#[test]
fn diff_sqlite_migration_directories() {
    let base_dir = tempfile::tempdir().unwrap();
    let base_dir_2 = tempfile::tempdir().unwrap();

    let migrations_lock_path = base_dir.path().join("migration_lock.toml");
    std::fs::write(migrations_lock_path, "provider = \"sqlite\"").unwrap();
    let migrations_lock_path = base_dir_2.path().join("migration_lock.toml");
    std::fs::write(migrations_lock_path, "provider = \"sqlite\"").unwrap();

    let migrations_list = list_migrations(&base_dir.keep()).unwrap();
    let migrations_list_2 = list_migrations(&base_dir_2.keep()).unwrap();

    let params = DiffParams {
        exit_code: None,
        from: DiffTarget::Migrations(migrations_list),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Migrations(migrations_list_2),
        filters: SchemaFilter::default(),
    };

    tok(schema_core::schema_api(None, None).unwrap().diff(params)).unwrap();
    // it's ok!
}

#[test]
fn diffing_mongo_schemas_works() {
    let tempdir = tempfile::tempdir().unwrap();

    let from = r#"
        datasource db {
            provider = "mongodb"
            url = "mongo+srv://test"
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
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: from_file.to_string_lossy().into_owned(),
                content: from.to_string(),
            }],
        }),
        shadow_database_url: None,
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: to_file.to_string_lossy().into_owned(),
                content: to.to_string(),
            }],
        }),
        script: false,
        filters: SchemaFilter::default(),
    };

    let expected_printed_messages = expect![[r#"
        [+] Collection `TestModel2`
        [+] Index `TestModel_names_idx` on ({"names":1})
    "#]];

    expected_printed_messages.assert_eq(&diff_output(params));
}

#[test]
fn diffing_two_schema_datamodels_with_missing_datasource_env_vars() {
    for provider in ["sqlite", "postgresql", "postgres", "mysql", "sqlserver"] {
        let schema_a = format!(
            r#"
            datasource db {{
                provider = "{provider}"
                url = env("HELLO_THIS_ENV_VAR_IS_NOT_D3F1N3D")
            }}
        "#
        );

        let schema_b = format!(
            r#"
            datasource db {{
                provider = "{provider}"
                url = env("THIS_ENV_VAR_DO3S_N0T_EXiST_EITHER")
            }}

            model Particle {{
                id Int @id
            }}
        "#
        );

        let tmpdir = tempfile::tempdir().unwrap();
        let schema_a_path = write_file_to_tmp(&schema_a, &tmpdir, "schema_a");
        let schema_b_path = write_file_to_tmp(&schema_b, &tmpdir, "schema_b");

        let expected = expect![[r#"

            [+] Added tables
              - Particle
        "#]];
        expected.assert_eq(&diff_output(DiffParams {
            exit_code: None,
            from: DiffTarget::SchemaDatamodel(SchemasContainer {
                files: vec![SchemaContainer {
                    path: schema_a_path.to_str().unwrap().to_owned(),
                    content: schema_a.to_string(),
                }],
            }),
            script: false,
            shadow_database_url: None,
            to: DiffTarget::SchemaDatamodel(SchemasContainer {
                files: vec![SchemaContainer {
                    path: schema_b_path.to_str().unwrap().to_owned(),
                    content: schema_b.to_string(),
                }],
            }),
            filters: SchemaFilter::default(),
        }))
    }
}

#[test]
fn diff_with_exit_code_and_empty_diff_returns_zero() {
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model Puppy {
            id Int @id
            name String
        }
    "#;

    let tmpdir = tempfile::tempdir().unwrap();
    let path = write_file_to_tmp(schema, &tmpdir, "schema.prisma");

    let (result, diff) = diff_result(DiffParams {
        exit_code: Some(true),
        from: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: path.to_str().unwrap().to_owned(),
                content: schema.to_string(),
            }],
        }),
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: path.to_str().unwrap().to_owned(),
                content: schema.to_string(),
            }],
        }),
        script: false,
        shadow_database_url: None,
        filters: SchemaFilter::default(),
    });

    assert_eq!(result.exit_code, 0);
    let expected_diff = expect![[r#"
        No difference detected.
    "#]];
    expected_diff.assert_eq(&diff);
}

#[test]
fn diff_with_exit_code_and_non_empty_diff_returns_two() {
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model Puppy {
            id Int @id
            name String
        }
    "#;

    let tmpdir = tempfile::tempdir().unwrap();
    let path = write_file_to_tmp(schema, &tmpdir, "schema.prisma");

    let (result, diff) = diff_result(DiffParams {
        exit_code: Some(true),
        from: DiffTarget::Empty,
        to: DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: path.to_str().unwrap().to_owned(),
                content: schema.to_string(),
            }],
        }),
        script: false,
        shadow_database_url: None,
        filters: SchemaFilter::default(),
    });

    assert_eq!(result.exit_code, 2);
    let expected_diff = expect![[r#"

        [+] Added tables
          - Puppy
    "#]];
    expected_diff.assert_eq(&diff);
}

#[test]
fn diff_with_non_existing_sqlite_database_from_url() {
    let expected = expect![[r#"
        Database `db.sqlite` does not exist
    "#]];
    let tmpdir = tempfile::tempdir().unwrap();

    let error = diff_error(DiffParams {
        exit_code: Some(true),
        from: DiffTarget::Empty,
        script: false,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer {
            url: format!("file:{}", tmpdir.path().join("db.sqlite").to_string_lossy()),
        }),
        filters: SchemaFilter::default(),
    });

    let error = error
        .replace(tmpdir.path().to_str().unwrap(), "<the-tmpdir-path>")
        .replace(std::path::MAIN_SEPARATOR, "/"); // normalize windows paths

    expected.assert_eq(&error);
}

#[test]
fn diff_with_non_existing_sqlite_database_from_datasource() {
    let expected = expect![[r#"
        Database `assume.sqlite` does not exist
    "#]];

    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = "file:/this/file/doesnt/exist/we/assume.sqlite"
        }
    "#;
    let tmpdir = tempfile::tempdir().unwrap();

    let schema_path = write_file_to_tmp(schema, &tmpdir, "schema.prisma");

    let error = diff_error(DiffParams {
        exit_code: Some(true),
        from: DiffTarget::Empty,
        script: false,
        shadow_database_url: None,
        to: DiffTarget::SchemaDatasource(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: schema.to_string(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        filters: SchemaFilter::default(),
    });

    if cfg!(target_os = "windows") {
        return; // path in error looks different
    }

    expected.assert_eq(&error);
}

#[test_connector]
fn from_multi_file_schema_datasource_to_url(mut api: TestApi) {
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{base_dir_str}/first_db.sqlite");
    let second_url = format!("file:{base_dir_str}/second_db.sqlite");

    tok(async {
        let q = quaint::single::Quaint::new(&first_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cows ( id INTEGER PRIMARY KEY, moos BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    tok(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cats ( id INTEGER PRIMARY KEY, meows BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    let schema_a = format!(
        r#"
          datasource db {{
              provider = "sqlite"
              url = "{}"
          }}
        "#,
        first_url.replace('\\', "\\\\")
    );
    let schema_a_path = write_file_to_tmp(&schema_a, &base_dir, "a.prisma");

    let schema_b = r#"
          model cats {
            id Int @id
            meows Boolean
          }
        "#;
    let schema_b_path = write_file_to_tmp(schema_b, &base_dir, "b.prisma");

    let files = to_schema_containers(&[
        (schema_a_path.to_string_lossy().into_owned(), &schema_a),
        (schema_b_path.to_string_lossy().into_owned(), schema_b),
    ]);

    let input = DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatasource(SchemasWithConfigDir {
            files,
            config_dir: base_dir.path().to_string_lossy().into_owned(),
        }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
        filters: SchemaFilter::default(),
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
fn from_multi_file_schema_datamodel_to_url(mut api: TestApi) {
    let host = Arc::new(TestConnectorHost::default());
    api.connector.set_host(host.clone());

    let base_dir = tempfile::TempDir::new().unwrap();
    let base_dir_str = base_dir.path().to_string_lossy();
    let first_url = format!("file:{base_dir_str}/first_db.sqlite");
    let second_url = format!("file:{base_dir_str}/second_db.sqlite");

    tok(async {
        let q = quaint::single::Quaint::new(&second_url).await.unwrap();
        q.raw_cmd("CREATE TABLE cats ( id INTEGER PRIMARY KEY, meows BOOLEAN DEFAULT true );")
            .await
            .unwrap();
    });

    let from_files = {
        let schema_a = format!(
            r#"
              datasource db {{
                  provider = "sqlite"
                  url = "{}"
              }}

              model cows {{
                id Int @id
                meows Boolean
              }}
            "#,
            first_url.replace('\\', "\\\\")
        );
        let schema_a_path = write_file_to_tmp(&schema_a, &base_dir, "a.prisma");

        let schema_b = r#"
              model dogs {
                id Int @id
                wouaf Boolean
              }
            "#;
        let schema_b_path = write_file_to_tmp(schema_b, &base_dir, "b.prisma");

        to_schema_containers(&[
            (schema_a_path.to_string_lossy().into_owned(), &schema_a),
            (schema_b_path.to_string_lossy().into_owned(), schema_b),
        ])
    };

    let input = DiffParams {
        exit_code: None,
        from: DiffTarget::SchemaDatamodel(SchemasContainer { files: from_files }),
        script: true,
        shadow_database_url: None,
        to: DiffTarget::Url(UrlContainer { url: second_url }),
        filters: SchemaFilter::default(),
    };

    api.diff(input).unwrap();

    let expected_printed_messages = expect![[r#"
        [
            "-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"cows\";\nPRAGMA foreign_keys=on;\n\n-- DropTable\nPRAGMA foreign_keys=off;\nDROP TABLE \"dogs\";\nPRAGMA foreign_keys=on;\n\n-- CreateTable\nCREATE TABLE \"cats\" (\n    \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,\n    \"meows\" BOOLEAN DEFAULT true\n);\n",
        ]
    "#]];
    expected_printed_messages.assert_debug_eq(&host.printed_messages.lock().unwrap());
}

// Call diff, and expect it to error. Return the error.
pub(crate) fn diff_error(params: DiffParams) -> String {
    let api = schema_core::schema_api(None, None).unwrap();
    let result = test_setup::runtime::run_with_thread_local_runtime(api.diff(params));
    result.unwrap_err().to_string()
}

// Call diff, and expect it to succeed. Return the result and what would be printed to stdout.
pub(crate) fn diff_result(params: DiffParams) -> (DiffResult, String) {
    let host = Arc::new(TestConnectorHost::default());
    let api = schema_core::schema_api(None, Some(host.clone())).unwrap();
    let result = test_setup::runtime::run_with_thread_local_runtime(api.diff(params)).unwrap();
    let printed_messages = host.printed_messages.lock().unwrap();
    assert!(printed_messages.len() == 1, "{printed_messages:?}");
    (result, printed_messages[0].clone())
}

// Call diff, and expect it to succeed. Return what would be printed to stdout.
fn diff_output(params: DiffParams) -> String {
    diff_result(params).1
}

pub(crate) fn write_file_to_tmp(contents: &str, tempdir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let tempfile_path = tempdir.path().join(name);
    std::fs::write(&tempfile_path, contents.as_bytes()).unwrap();
    tempfile_path
}
