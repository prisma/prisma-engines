use std::{fs::File, io::Write};

use indoc::indoc;
use migration_engine_tests::{multi_engine_test_api::TestApi, sqlite_test_url, TestResult};
use test_macros::test_connectors;
use user_facing_errors::{migration_engine::ProviderSwitchedError, UserFacingError};

#[test_connectors(tags("postgres"))]
async fn create_migration_with_new_provider_errors(api: TestApi) -> TestResult {
    api.initialize().await?;

    let dm = r#"
    datasource db {
        provider = "postgresql"
        url = "postgres://unreachable"
    }

    model Cat {
        id Int @id
    }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    let engine = api
        .new_engine_with_connection_strings(api.connection_string(), None)
        .await?;

    engine
        .create_migration("01init", dm, &migrations_directory)
        .send()
        .await?;

    let dm2 = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model Cat {
            id Int @id
        }
    "#;

    let sqlite_engine = api
        .new_engine_with_connection_strings(&sqlite_test_url("migratelocktest"), None)
        .await?;

    let err = sqlite_engine
        .create_migration("02switchprovider", dm2, &migrations_directory)
        .send()
        .await
        .unwrap_err()
        .render_user_facing();

    let err = err.as_known().unwrap();

    assert_eq!(err.error_code, ProviderSwitchedError::ERROR_CODE);
    assert!(err.message.contains("postgresql"));
    assert!(err.message.contains("sqlite"), "{:?}", err);

    Ok(())
}

#[test_connectors(tags("postgres"))]
async fn migration_lock_with_different_comment_shapes_work(api: TestApi) -> TestResult {
    api.initialize().await?;

    let dm = r#"
    datasource db {
        provider = "postgresql"
        url = "postgres://unreachable"
    }

    model Cat {
        id Int @id
    }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

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

    let engine = api
        .new_engine_with_connection_strings(api.connection_string(), None)
        .await?;

    for contents in contents {
        let span = tracing::info_span!("Contents", contents = contents);
        let _span = span.enter();

        let mut file = File::create(&migration_lock_path)?;

        file.write_all(contents.as_bytes())?;

        let err = engine
            .create_migration("01init", dm, &migrations_directory)
            .send()
            .await
            .unwrap_err()
            .render_user_facing();

        let err = err.as_known().unwrap();

        assert_eq!(err.error_code, ProviderSwitchedError::ERROR_CODE);
        assert!(err.message.contains("postgresql"));
        assert!(err.message.contains("sqlite"), "{:?}", err);
    }

    Ok(())
}
