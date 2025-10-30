use sql_migration_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
fn failing_migration_after_migration_dropping_data(api: TestApi) {
    let migrations = &[
        r#"
            CREATE TABLE "Dog" (
                id              SERIAL PRIMARY KEY,
                name            TEXT NOT NULL,
                is_good_dog     BOOLEAN NOT NULL DEFAULT TRUE
            );

            INSERT INTO "Dog" (name, is_good_dog) VALUES
                ('snoopy', true),
                ('marmaduke', true),
                ('pluto', true),
                ('dingo', true),
                ('itzi', true),
                ('mugi', true)
                ;
        "#,
        r#"
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
        "#,
        r#"
            ALTER TABLE "Dog" ALTER COLUMN is_good_dog TYPE INTEGER;
        "#,
    ];
    let dir = write_migrations(migrations);
    let err = api.apply_migrations(&dir).send_unwrap_err();
    let expectation = expect![[r#"
        A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

        Migration name:   2

        Database error code: 42703

        Database error:
        ERROR: column "is_good_dog" does not exist

        DbError { severity: "ERROR", parsed_severity: Some(Error), code: SqlState(E42703), message: "column \"is_good_dog\" does not exist", detail: None, hint: None, position: None, where_: None, schema: None, table: None, column: None, datatype: None, constraint: None, file: Some("column_resolver.go"), line: Some(196), routine: Some("NewUndefinedColumnError") }
    "#]];
    expectation.assert_eq(err.message().unwrap());
}

#[test_connector(tags(CockroachDb))]
fn failing_step_in_migration_dropping_data(api: TestApi) {
    let migrations = &[
        r#"
            CREATE TABLE "Dog" (
                id              SERIAL PRIMARY KEY,
                name            TEXT NOT NULL,
                is_good_dog     BOOLEAN NOT NULL DEFAULT TRUE
            );

            INSERT INTO "Dog" (name, is_good_dog) VALUES
                ('snoopy', true),
                ('marmaduke', true),
                ('pluto', true),
                ('dingo', true),
                ('itzi', true),
                ('mugi', true)
                ;
        "#,
        r#"
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
            ALTER TABLE "Dog" ALTER COLUMN is_good_dog TYPE INTEGER;
        "#,
    ];
    let dir = write_migrations(migrations);
    let err = api.apply_migrations(&dir).send_unwrap_err();
    let expectation = expect![[r#"
        A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

        Migration name:   1

        Database error code: 42703

        Database error:
        ERROR: column "is_good_dog" does not exist

        DbError { severity: "ERROR", parsed_severity: Some(Error), code: SqlState(E42703), message: "column \"is_good_dog\" does not exist", detail: None, hint: None, position: None, where_: None, schema: None, table: None, column: None, datatype: None, constraint: None, file: Some("column_resolver.go"), line: Some(196), routine: Some("NewUndefinedColumnError") }
    "#]];
    expectation.assert_eq(err.message().unwrap());
}

#[test_connector(tags(CockroachDb))]
fn multiple_alter_tables_in_one_migration_works(api: TestApi) {
    let migrations = &[
        r#"
            CREATE TABLE "Dog" (
                id              SERIAL PRIMARY KEY,
                name            TEXT NOT NULL,
                is_good_dog     BOOLEAN NOT NULL DEFAULT TRUE
            );

            INSERT INTO "Dog" (name, is_good_dog) VALUES
                ('snoopy', true),
                ('marmaduke', true),
                ('pluto', true),
                ('dingo', true),
                ('itzi', true),
                ('mugi', true)
                ;
        "#,
        r#"
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
            ALTER TABLE "Dog" ADD COLUMN is_good_dog INTEGER NOT NULL DEFAULT 100;
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
            ALTER TABLE "Dog" ADD COLUMN is_good_dog INTEGER NOT NULL DEFAULT 100;
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
            ALTER TABLE "Dog" ADD COLUMN is_good_dog INTEGER NOT NULL DEFAULT 100;
        "#,
    ];
    let dir = write_migrations(migrations);
    api.apply_migrations(&dir).send_sync();
}

#[test_connector(tags(CockroachDb))]
fn multiple_alter_tables_in_multiple_migration_works(api: TestApi) {
    let migrations = &[
        r#"
            CREATE TABLE "Dog" (
                id              SERIAL PRIMARY KEY,
                name            TEXT NOT NULL,
                is_good_dog     BOOLEAN NOT NULL DEFAULT TRUE
            );

            INSERT INTO "Dog" (name, is_good_dog) VALUES
                ('snoopy', true),
                ('marmaduke', true),
                ('pluto', true),
                ('dingo', true),
                ('itzi', true),
                ('mugi', true)
                ;
        "#,
        r#"
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
        "#,
        r#"
            ALTER TABLE "Dog" ADD COLUMN is_good_dog INTEGER NOT NULL DEFAULT 100;
        "#,
        r#"
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
        "#,
        r#"
            ALTER TABLE "Dog" ADD COLUMN is_good_dog INTEGER NOT NULL DEFAULT 100;
        "#,
        r#"
            ALTER TABLE "Dog" DROP COLUMN is_good_dog;
        "#,
        r#"
            ALTER TABLE "Dog" ADD COLUMN is_good_dog INTEGER NOT NULL DEFAULT 100;
        "#,
    ];
    let dir = write_migrations(migrations);
    api.apply_migrations(&dir).send_sync();
}

#[test_connector(tags(CockroachDb))]
fn syntax_errors_return_error_position(api: TestApi) {
    let migrations = &[r#"
            CREATE TABLE "Dog" (
                id              SERIAL PRIMARY KEY,
                name            TEXT NOT NULL
                is_good_dog     BOOLEAN NOT NULL DEFAULT TRUE
            );
        "#];
    let dir = write_migrations(migrations);
    let err = api.apply_migrations(&dir).send_unwrap_err();
    let expectation = expect![[r#"
        A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

        Migration name:   0

        Database error code: 42601

        Database error:
        ERROR: at or near "is_good_dog": syntax error
        DETAIL: source SQL:
        CREATE TABLE "Dog" (
                        id              SERIAL PRIMARY KEY,
                        name            TEXT NOT NULL
                        is_good_dog     BOOLEAN NOT NULL DEFAULT TRUE
                        ^
        HINT: try \h CREATE TABLE

        DbError { severity: "ERROR", parsed_severity: Some(Error), code: SqlState(E42601), message: "at or near \"is_good_dog\": syntax error", detail: Some("source SQL:\nCREATE TABLE \"Dog\" (\n                id              SERIAL PRIMARY KEY,\n                name            TEXT NOT NULL\n                is_good_dog     BOOLEAN NOT NULL DEFAULT TRUE\n                ^"), hint: Some("try \\h CREATE TABLE"), position: None, where_: None, schema: None, table: None, column: None, datatype: None, constraint: None, file: Some("lexer.go"), line: Some(271), routine: Some("Error") }
    "#]];
    expectation.assert_eq(err.message().unwrap());
}

fn write_migrations(migrations: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (idx, migration) in migrations.iter().enumerate() {
        let migration_dir = dir.path().join(format!("{idx:3}"));
        std::fs::create_dir(&migration_dir).unwrap();
        let migration_path = migration_dir.join("migration.sql");
        std::fs::write(&migration_path, migration).unwrap();
    }
    dir
}
