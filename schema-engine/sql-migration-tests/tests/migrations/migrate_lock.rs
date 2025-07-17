use indoc::indoc;
use sql_migration_tests::multi_engine_test_api::*;
use std::{fs::File, io::Write};
use test_macros::test_connector;
use user_facing_errors::{UserFacingError, schema_engine::ProviderSwitchedError};

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn create_migration_with_new_provider_errors(api: TestApi) {
    let dm = r#"
    datasource db {
        provider = "postgresql"
        url = "postgres://unreachable"
    }

    model Cat {
        id Int @id
    }
    "#;

    let migrations_directory = api.create_migrations_directory();
    let mut engine = api.new_engine_with_connection_strings(api.connection_string().to_owned(), None);

    engine.create_migration("01init", dm, &migrations_directory).send_sync();

    let dm2 = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model Cat {
            id Int @id
        }
    "#;

    let mut sqlite_engine = api.new_engine_with_connection_strings(sqlite_test_url("migratelocktest"), None);

    let err = sqlite_engine
        .create_migration("02switchprovider", dm2, &migrations_directory)
        .send_unwrap_err()
        .to_user_facing();

    let err = err.as_known().unwrap();

    assert_eq!(err.error_code, ProviderSwitchedError::ERROR_CODE);
    assert!(err.message.contains("postgresql"));
    assert!(err.message.contains("sqlite"), "{err:?}");
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn migration_lock_with_different_comment_shapes_work(api: TestApi) {
    let dm = r#"
    datasource db {
        provider = "postgresql"
        url = "postgres://unreachable"
    }

    model Cat {
        id Int @id
    }
    "#;

    let migrations_directory = api.create_migrations_directory();

    let contents = &[
        indoc!(
            r#"
            # acd
            # def
            provider = "sqlite"
            "#
        ),
        indoc!(
            r#"
            # abc
            provider = "sqlite"
            "#
        ),
        r#"provider = "sqlite""#,
        r#"provider = "sqlite"
        # heh"#,
    ];

    let migration_lock_path = migrations_directory.path().join("migration_lock.toml");

    let mut engine = api.new_engine_with_connection_strings(api.connection_string().to_owned(), None);

    for contents in contents {
        let span = tracing::info_span!("Contents", contents = contents);
        let _span = span.enter();

        let mut file = File::create(&migration_lock_path).unwrap();

        file.write_all(contents.as_bytes()).unwrap();

        let err = engine
            .create_migration("01init", dm, &migrations_directory)
            .send_unwrap_err()
            .to_user_facing();

        let err = err.as_known().unwrap();

        assert_eq!(err.error_code, ProviderSwitchedError::ERROR_CODE);
        assert!(err.message.contains("postgresql"));
        assert!(err.message.contains("sqlite"), "{err:?}");
    }
}
