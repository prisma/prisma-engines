use migration_engine_tests::{multi_engine_test_api::TestApi, TestResult};
use quaint::{prelude::Queryable, single::Quaint};
use test_macros::test_connectors;

#[test_connectors(tags("postgres"), log = "debug")]
async fn shadow_db_url_can_be_configured_on_postgres(api: TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let mut url: url::Url = api.connection_string().parse()?;

    let dm1 = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    let dm2 = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)

            meowFrequency Float
        }
    "#;

    // Create the database, a first migration and the test user.
    {
        let admin_connection = api.initialize().await?;

        {
            let engine = api.new_engine().await?;

            engine
                .create_migration("01initcats", dm1, &migrations_directory)
                .send()
                .await?;
        }

        admin_connection
            .raw_cmd("DROP DATABASE IF EXISTS testshadowdb0001")
            .await?;
        admin_connection.raw_cmd("CREATE DATABASE testshadowdb0001").await.ok();

        let create_user = r#"
            DROP USER IF EXISTS shadowdbconfigtestuser;
            CREATE USER shadowdbconfigtestuser PASSWORD '1234batman' LOGIN;
            GRANT USAGE, CREATE ON SCHEMA "prisma-tests" TO shadowdbconfigtestuser;
            GRANT ALL PRIVILEGES ON DATABASE "testshadowdb0001" TO shadowdbconfigtestuser;
        "#;

        admin_connection.raw_cmd(&create_user).await?;

        let mut shadow_db_url = url.clone();
        shadow_db_url.set_path("testshadowdb0001");

        let shadow_db_connection = Quaint::new(&shadow_db_url.to_string()).await?;

        shadow_db_connection
            .raw_cmd("CREATE SCHEMA \"prisma-tests\"; GRANT USAGE, CREATE ON SCHEMA \"prisma-tests\" TO shadowdbconfigtestuser")
            .await?;
    }

    let test_user_connection_string = {
        url.set_username("shadowdbconfigtestuser").unwrap();
        url.set_password(Some("1234batman")).unwrap();
        url.to_string()
    };

    let custom_shadow_db_url = {
        url.set_path("testshadowdb0001");
        url.to_string()
    };

    // Check that the test user can't drop databases.
    {
        let test_user_connection = Quaint::new(&test_user_connection_string).await?;

        let err = test_user_connection
            .raw_cmd("CREATE DATABASE shadowdburltest83429")
            .await
            .unwrap_err();

        assert_eq!(err.original_code().unwrap(), "42501"); // insufficient_privilege (https://www.postgresql.org/docs/current/errcodes-appendix.html)
    }

    // Check that commands using the shadow database work.
    {
        let engine = api
            .new_engine_with_connection_strings(&test_user_connection_string, Some(custom_shadow_db_url))
            .await?;

        engine
            .apply_migrations(&migrations_directory)
            .send()
            .await?
            .assert_applied_migrations(&["01initcats"])?;

        engine
            .create_migration("02addMeowFrequency", dm2, &migrations_directory)
            .send()
            .await?;

        engine
            .apply_migrations(&migrations_directory)
            .send()
            .await?
            .assert_applied_migrations(&["02addMeowFrequency"])?;

        engine
            .assert_schema()
            .await?
            .assert_tables_count(2)?
            .assert_has_table("_prisma_migrations")?
            .assert_table("Cat", |table| table.assert_has_column("meowFrequency"))?;
    }

    Ok(())
}
