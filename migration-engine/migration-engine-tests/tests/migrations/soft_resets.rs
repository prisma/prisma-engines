use migration_core::SchemaPushInput;
use migration_engine_tests::{multi_engine_test_api::TestApi, TestResult};
use quaint::{prelude::Queryable, single::Quaint};
use test_macros::test_each_connector;

#[test_each_connector(tags("postgres"), log = "debug")]
async fn soft_resets_work_on_postgres(api: &TestApi) -> TestResult {
    let mut url: url::Url = api.connection_string().parse()?;
    dbg!(&url);

    // Create the database and the test user.
    {
        let admin_connection = api.initialize().await?;

        let create_user = r#"
            DROP USER IF EXISTS softresetstestuser;
            CREATE USER softresetstestuser PASSWORD '1234batman' LOGIN;
            GRANT USAGE, CREATE ON SCHEMA "prisma-tests" TO softresetstestuser;
        "#;

        admin_connection.raw_cmd(&create_user).await?;
    }

    let test_user_connection_string = {
        url.set_username("softresetstestuser").unwrap();
        url.set_password(Some("1234batman")).unwrap();
        url.to_string()
    };

    // Check that the test user can't drop databases.
    {
        let test_user_connection = Quaint::new(&test_user_connection_string).await?;

        let err = test_user_connection
            .raw_cmd(&format!(r#"DROP DATABASE {}"#, api.test_fn_name()))
            .await
            .unwrap_err();

        assert_eq!(err.original_code().unwrap(), "42501"); // insufficient_privilege (https://www.postgresql.org/docs/current/errcodes-appendix.html)
    }

    dbg!(&test_user_connection_string);
    let engine = api
        .new_engine_with_connection_string(&test_user_connection_string)
        .await?;

    engine
        .schema_push(&SchemaPushInput {
            schema: r#"
            model Cat {
                id Int @id
                litterConsumption Int
                hungry Boolean @default(true)
            }
        "#
            .into(),
            force: true,
            assume_empty: false,
        })
        .await?;

    engine.reset(&()).await?;

    Ok(())
}
