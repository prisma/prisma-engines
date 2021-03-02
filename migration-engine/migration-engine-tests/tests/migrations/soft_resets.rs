use connection_string::JdbcString;
use migration_engine_tests::{multi_engine_test_api::TestApi, TestResult};
use quaint::{prelude::Queryable, single::Quaint};
use test_macros::test_connectors;

#[test_connectors(tags("postgres"))]
async fn soft_resets_work_on_postgres(api: TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let mut url: url::Url = api.connection_string().parse()?;

    let dm = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    // Create the database, a first migration and the test user.
    {
        let admin_connection = api.initialize().await?;

        api.new_engine()
            .await?
            .create_migration("01init", dm, &migrations_directory)
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

    // Check that the soft reset works with migrations, then with schema push.
    {
        let engine = api
            .new_engine_with_connection_strings(&test_user_connection_string, None)
            .await?;

        engine
            .apply_migrations(&migrations_directory)
            .send()
            .await?
            .assert_applied_migrations(&["01init"])?;

        let add_view = format!(
            r#"CREATE VIEW "{0}"."catcat" AS SELECT * FROM "{0}"."Cat" LIMIT 2"#,
            engine.schema_name(),
        );

        engine.raw_cmd(&add_view).await?;

        engine
            .assert_schema()
            .await?
            .assert_tables_count(2)?
            .assert_has_table("_prisma_migrations")?
            .assert_has_table("Cat")?;

        engine.reset().send().await?;
        engine.assert_schema().await?.assert_tables_count(0)?;

        engine
            .schema_push(dm)
            .send()
            .await?
            .assert_has_executed_steps()?
            .assert_green()?;

        engine
            .assert_schema()
            .await?
            .assert_tables_count(1)?
            .assert_has_table("Cat")?;

        engine.reset().send().await?;
        engine.assert_schema().await?.assert_tables_count(0)?;
    }

    Ok(())
}

#[test_connectors(tags("mssql"))]
async fn soft_resets_work_on_sql_server(api: TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;

    let mut url: JdbcString = format!("jdbc:{}", api.connection_string()).parse()?;

    let dm = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    // Create the database, a first migration and the test user.
    {
        let admin_connection = api.initialize().await?;

        api.new_engine()
            .await?
            .create_migration("01init", dm, &migrations_directory)
            .send()
            .await?;

        let create_database = r#"
            IF(DB_ID(N'resetTest') IS NOT NULL)
            BEGIN
                DROP DATABASE [resetTest]
            END;

            CREATE DATABASE [resetTest];
        "#;

        let create_user = r#"
            USE [resetTest];

            IF EXISTS (SELECT loginname from dbo.syslogins 
                WHERE name = 'softresetstestuser')
            BEGIN
                DROP LOGIN softresetstestuser;
            END;

            CREATE LOGIN softresetstestuser WITH PASSWORD = 'Password123Password123';
            CREATE USER softresetstestuser FROM LOGIN softresetstestuser;
            GRANT CONTROL TO softresetstestuser;
            REVOKE ALTER TO softresetstestuser;
        "#;

        admin_connection.raw_cmd(create_database).await?;
        admin_connection.raw_cmd(create_user).await?;
    }

    let test_user_connection_string = {
        let properties = url.properties_mut();

        properties.insert("user".into(), "softresetstestuser".into());
        properties.insert("password".into(), "Password123Password123".into());
        properties.insert("database".into(), "resetTest".into());

        url.to_string()
    };

    // Check that the test user can't drop databases.
    {
        let test_user_connection = Quaint::new(&test_user_connection_string).await?;

        let err = test_user_connection
            .raw_cmd(&format!(r#"DROP DATABASE {}"#, api.test_fn_name()))
            .await
            .unwrap_err();

        // insufficent privilege
        // https://docs.microsoft.com/en-us/sql/relational-databases/errors-events/database-engine-events-and-errors
        assert_eq!(err.original_code().unwrap(), "3701");
    }

    // Check that the soft reset works with migrations, then with schema push.
    {
        let engine = api
            .new_engine_with_connection_strings(&test_user_connection_string, None)
            .await?;

        let create_schema = format!("CREATE SCHEMA [{}];", engine.schema_name());
        engine.raw_cmd(&create_schema).await?;

        engine
            .apply_migrations(&migrations_directory)
            .send()
            .await?
            .assert_applied_migrations(&["01init"])?;

        let add_view = format!(
            r#"CREATE VIEW [{0}].[catcat] AS SELECT * FROM [{0}].[Cat]"#,
            engine.schema_name(),
        );

        engine.raw_cmd(&add_view).await?;

        engine
            .assert_schema()
            .await?
            .assert_tables_count(2)?
            .assert_has_table("_prisma_migrations")?
            .assert_has_table("Cat")?;

        engine.reset().send().await?;
        engine.assert_schema().await?.assert_tables_count(0)?;

        engine
            .schema_push(dm)
            .send()
            .await?
            .assert_has_executed_steps()?
            .assert_green()?;

        engine
            .assert_schema()
            .await?
            .assert_tables_count(1)?
            .assert_has_table("Cat")?;

        engine.reset().send().await?;
        engine.assert_schema().await?.assert_tables_count(0)?;
    }

    Ok(())
}

/// MySQL 5.6 doesn't have `DROP USER IF EXISTS`...
#[test_connectors(tags("mysql"), ignore("mysql_5_6"))]
async fn soft_resets_work_on_mysql(api: TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let mut url: url::Url = api.connection_string().parse()?;
    let admin_connection = api.initialize().await?;

    let dm = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    {
        let engine = api.new_engine().await?;

        engine
            .create_migration("01init", dm, &migrations_directory)
            .send()
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
    }

    {
        let create_user = format!(
            r#"
            DROP USER IF EXISTS 'softresetstestuser'@'%';
            CREATE USER 'softresetstestuser'@'%' IDENTIFIED BY '1234batman';
            GRANT USAGE, CREATE ON TABLE `{0}`.* TO 'softresetstestuser'@'%';
            GRANT DROP ON TABLE `{0}`.`Cat` TO 'softresetstestuser'@'%';
            GRANT DROP ON TABLE `{0}`.`_prisma_migrations` TO 'softresetstestuser'@'%';
            FLUSH PRIVILEGES;
        "#,
            api.test_fn_name()
        );

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
            .raw_cmd(&format!(r#"DROP DATABASE `{}`"#, api.test_fn_name()))
            .await
            .unwrap_err();

        // insufficient_privilege
        // https://docs.oracle.com/cd/E19078-01/mysql/mysql-refman-5.1/error-handling.html
        assert_eq!(err.original_code().unwrap(), "1044");
    }

    // Check that the soft reset works with migrations, then with schema push.
    {
        let engine = api
            .new_engine_with_connection_strings(&test_user_connection_string, None)
            .await?;

        engine.reset().send().await?;
        engine.assert_schema().await?.assert_tables_count(0)?;

        engine
            .schema_push(dm)
            .send()
            .await?
            .assert_has_executed_steps()?
            .assert_green()?;

        engine
            .assert_schema()
            .await?
            .assert_tables_count(1)?
            .assert_has_table("Cat")?;

        engine.reset().send().await?;
        engine.assert_schema().await?.assert_tables_count(0)?;
    }

    Ok(())
}
