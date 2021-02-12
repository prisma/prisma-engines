use migration_engine_tests::{multi_engine_test_api::TestApi, TestResult};
use quaint::{prelude::Queryable, single::Quaint};
use test_macros::test_each_connector;

#[test_each_connector(tags("postgres"), log = "debug")]
async fn soft_resets_work_on_postgres(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let mut url: url::Url = api.connection_string().parse()?;

    // Create the database, a first migration and the test user.
    {
        let admin_connection = api.initialize().await?;

        api.new_engine()
            .await?
            .create_migration(
                "01init",
                r#"
                model Cat {
                    id Int @id
                    litterConsumption Int
                    hungry Boolean @default(true)
                }
            "#,
                &migrations_directory,
            )
            .send()
            .await?;

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

    // Check that the soft reset works.
    {
        let engine = api
            .new_engine_with_connection_string(&test_user_connection_string)
            .await?;

        engine
            .apply_migrations(&migrations_directory)
            .send()
            .await?
            .assert_applied_migrations(&["01init"])?;

        engine
            .assert_schema()
            .await?
            .assert_tables_count(2)?
            .assert_has_table("_prisma_migrations")?
            .assert_has_table("Cat")?;

        engine.reset().send().await?;

        engine.assert_schema().await?.assert_tables_count(0)?;
    }

    Ok(())
}
