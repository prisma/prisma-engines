use migration_engine_tests::{multi_engine_test_api::TestApi, sqlite_test_url, TestResult};
use test_macros::test_connectors;
use user_facing_errors::{migration_engine::ProviderSwitchedError, UserFacingError};

#[test_connectors(tags("postgres"))]
async fn create_migration_with_new_provider_errors(api: TestApi) -> TestResult {
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
            provider = "mysql"
            url = "mysql://unreachable"
        }

        model Cat {
            id Int @id
        }
    "#;

    let mysql_engine = api
        .new_engine_with_connection_strings(&sqlite_test_url("migratelocktest"), None)
        .await?;

    let err = mysql_engine
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
