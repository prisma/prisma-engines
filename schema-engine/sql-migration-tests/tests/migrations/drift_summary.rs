use expect_test::{expect, Expect};
use schema_core::json_rpc::types::SchemasContainer;
use sql_migration_tests::test_api::*;
use std::sync::Arc;

fn check(from: &str, to: &str, expectation: Expect) {
    let tmpdir = tempfile::tempdir().unwrap();
    let from_schema = write_file_to_tmp(from, &tmpdir, "from.prisma");
    let to_schema = write_file_to_tmp(to, &tmpdir, "to.prisma");

    let params = DiffParams {
        exit_code: None,
        from: schema_core::json_rpc::types::DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: from_schema.to_str().unwrap().to_owned(),
                content: from.to_string(),
            }],
        }),
        script: false,
        shadow_database_url: None,
        to: schema_core::json_rpc::types::DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![SchemaContainer {
                path: to_schema.to_str().unwrap().to_owned(),
                content: to.to_string(),
            }],
        }),
    };

    let host = Arc::new(TestConnectorHost::default());
    let api = schema_core::schema_api(None, Some(host.clone())).unwrap();
    test_setup::runtime::run_with_thread_local_runtime(api.diff(params)).unwrap();
    let printed_messages = host.printed_messages.lock().unwrap();
    assert!(printed_messages.len() == 1, "{printed_messages:?}");
    expectation.assert_eq(&printed_messages[0]);
}

fn write_file_to_tmp(contents: &str, tempdir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let tempfile_path = tempdir.path().join(name);
    std::fs::write(&tempfile_path, contents.as_bytes()).unwrap();
    tempfile_path
}

#[test]
fn empty_schemas() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }
        "#,
        r#"
        datasource db {
            provider = "postgresql"
            url = env("TEST_DATABASE_URL")
        }
        "#,
        expect![[r#"
            No difference detected.
        "#]],
    )
}

#[test]
fn additions_table() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }
        "#,
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
        }
        "#,
        expect![[r#"

            [+] Added tables
              - Cat
        "#]],
    );
}

#[test]
fn additions_column() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
        }
        "#,
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
            name String?
        }
        "#,
        expect![[r#"

            [*] Changed the `Cat` table
              [+] Added column `name`
        "#]],
    );
}

#[test]
fn additions_enum() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        expect![[r#"

            [+] Added enums
              - Color
        "#]],
    );
}

#[test]
fn additions_mixed() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        expect![[r#"

            [+] Added enums
              - Color

            [+] Added tables
              - Cat
        "#]],
    );
}

#[test]
fn deletions_table() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
        }
        "#,
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }
        "#,
        expect![[r#"

            [-] Removed tables
              - Cat
        "#]],
    );
}

#[test]
fn deletions_enum() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        expect![[r#"

            [-] Removed enums
              - Color
        "#]],
    );
}

#[test]
fn deletions_mixed() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        expect![[r#"

            [-] Removed enums
              - Color

            [-] Removed tables
              - Cat
        "#]],
    );
}

#[test]
fn deletions_column() {
    check(
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://localhost/testdb"
        }

        model Cat {
            id Int @id
            name String?
        }
        "#,
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://localhost/testdb"
        }

        model Cat {
            id Int @id
        }
        "#,
        expect![[r#"

            [*] Changed the `Cat` table
              [-] Removed column `name`
        "#]],
    );
}

#[test]
fn additions_and_deletions_mixed() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Dog {
            id Int @id
            name String
        }
        "#,
        expect![[r#"

            [+] Added tables
              - Dog

            [-] Removed enums
              - Color

            [-] Removed tables
              - Cat
        "#]],
    );
}

#[test]
fn multiple_changed_tables_and_enums() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        model Dog {
            id Int @id
            name String
            isGoodDog Boolean?
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Dog {
            id Int
            weight Int @unique
            isGoodDog Boolean @default(true)
        }

        model Cat {
            id Int @id
            vaccinated Boolean
        }

        enum Color {
            GREEN
            BLUE
            TRANSPARENT
        }

        enum DogType {
            GOOD_DOG
        }

        enum CatType {
            MEOW_MEOW
        }
        "#,
        expect![[r#"

            [+] Added enums
              - DogType
              - CatType

            [*] Changed the `Color` enum
              [+] Added variant `TRANSPARENT`
              [-] Removed variant `RED`

            [*] Changed the `Cat` table
              [-] Removed column `color`
              [+] Added column `vaccinated`

            [*] Changed the `Dog` table
              [-] Dropped the primary key on columns (id)
              [-] Removed column `name`
              [+] Added column `weight`
              [*] Altered column `isGoodDog` (changed from Nullable to Required, default changed from `None` to `Some(Value(Boolean(true)))`)
              [+] Added unique index on columns (weight)
        "#]],
    );
}
