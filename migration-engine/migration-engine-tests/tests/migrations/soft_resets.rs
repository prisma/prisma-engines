use connection_string::JdbcString;
use migration_engine_tests::multi_engine_test_api::*;
use quaint::{prelude::Queryable, single::Quaint};
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
fn soft_resets_work_on_postgres(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let mut url: url::Url = api.connection_string().parse().unwrap();

    let dm = r#"
    model Cat {
        id Int @id
        litterConsumption Int
        hungry Boolean @default(true)
    }
    "#;

    // Create the database, a first migration and the test user.
    {
        api.new_engine()
            .create_migration("01init", dm, &migrations_directory)
            .send_sync();

        let create_user = r#"
            DROP USER IF EXISTS softresetstestuser;
            CREATE USER softresetstestuser PASSWORD '1234batman' LOGIN;
            GRANT USAGE, CREATE ON SCHEMA "prisma-tests" TO softresetstestuser;
        "#;

        api.raw_cmd(create_user);
    }

    let test_user_connection_string = {
        url.set_username("softresetstestuser").unwrap();
        url.set_password(Some("1234batman")).unwrap();
        url.to_string()
    };

    // Check that the test user can't drop databases.
    {
        let test_user_connection = api.block_on(Quaint::new(&test_user_connection_string)).unwrap();

        let err = api
            .block_on(test_user_connection.raw_cmd(&format!(r#"DROP DATABASE {}"#, api.test_fn_name())))
            .unwrap_err();

        assert_eq!(err.original_code().unwrap(), "42501"); // insufficient_privilege (https://www.postgresql.org/docs/current/errcodes-appendix.html)
    }

    // Check that the soft reset works with migrations, then with schema push.
    {
        let engine = api.new_engine_with_connection_strings(&test_user_connection_string, None);

        engine
            .apply_migrations(&migrations_directory)
            .send_sync()
            .assert_applied_migrations(&["01init"]);

        let add_view = format!(
            r#"CREATE VIEW "{0}"."catcat" AS SELECT * FROM "{0}"."Cat" LIMIT 2"#,
            engine.schema_name(),
        );

        engine.raw_cmd(&add_view);

        engine
            .assert_schema()
            .assert_tables_count(2)
            .assert_has_table("_prisma_migrations")
            .assert_has_table("Cat");

        engine.reset().send_sync();
        engine.assert_schema().assert_tables_count(0);

        engine
            .schema_push(dm)
            .send()
            .assert_has_executed_steps()
            .assert_green_bang();

        engine.assert_schema().assert_tables_count(1).assert_has_table("Cat");

        engine.reset().send_sync();
        engine.assert_schema().assert_tables_count(0);
    }
}

#[test_connector(tags(Mssql))]
fn soft_resets_work_on_sql_server(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let mut url: JdbcString = format!("jdbc:{}", api.connection_string()).parse().unwrap();

    let dm = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    // Create the database, a first migration and the test user.
    {
        api.new_engine()
            .create_migration("01init", dm, &migrations_directory)
            .send_sync();

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

        api.raw_cmd(create_database);
        api.raw_cmd(create_user);
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
        let test_user_connection = api.block_on(Quaint::new(&test_user_connection_string)).unwrap();

        let err = api
            .block_on(test_user_connection.raw_cmd(&format!(r#"DROP DATABASE {}"#, api.test_fn_name())))
            .unwrap_err();

        // insufficent privilege
        // https://docs.microsoft.com/en-us/sql/relational-databases/errors-events/database-engine-events-and-errors
        assert_eq!(err.original_code().unwrap(), "3701");
    }

    // Check that the soft reset works with migrations, then with schema push.
    {
        let engine = api.new_engine_with_connection_strings(&test_user_connection_string, None);

        let create_schema = format!("CREATE SCHEMA [{}];", engine.schema_name());
        engine.raw_cmd(&create_schema);

        engine
            .apply_migrations(&migrations_directory)
            .send_sync()
            .assert_applied_migrations(&["01init"]);

        let add_view = format!(
            r#"CREATE VIEW [{0}].[catcat] AS SELECT * FROM [{0}].[Cat]"#,
            engine.schema_name(),
        );

        engine.raw_cmd(&add_view);

        let add_type = format!(r#"CREATE TYPE [{0}].[Litter] FROM int"#, engine.schema_name(),);

        engine.raw_cmd(&add_type);

        let add_table_with_type = format!(
            r#"CREATE TABLE [{0}].specialLitter (id int primary key, litterAmount [{0}].Litter)"#,
            engine.schema_name()
        );

        engine.raw_cmd(&add_table_with_type);

        engine
            .assert_schema()
            .assert_tables_count(3)
            .assert_has_table("_prisma_migrations")
            .assert_has_table("specialLitter")
            .assert_has_table("Cat");

        engine.reset().send_sync();
        engine.assert_schema().assert_tables_count(0);

        engine
            .schema_push(dm)
            .send()
            .assert_has_executed_steps()
            .assert_green_bang();

        engine.assert_schema().assert_tables_count(1).assert_has_table("Cat");

        engine.reset().send_sync();
        engine.assert_schema().assert_tables_count(0);
    }
}

// MySQL 5.6 doesn't have `DROP USER IF EXISTS`...
// Neither does Vitess
#[test_connector(tags(Mysql), exclude(Mysql56, Vitess))]
fn soft_resets_work_on_mysql(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let mut url: url::Url = api.connection_string().parse().unwrap();

    let dm = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    {
        let engine = api.new_engine();

        engine.create_migration("01init", dm, &migrations_directory).send_sync();

        engine
            .apply_migrations(&migrations_directory)
            .send_sync()
            .assert_applied_migrations(&["01init"]);

        engine
            .assert_schema()
            .assert_tables_count(2)
            .assert_has_table("_prisma_migrations")
            .assert_has_table("Cat");
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

        api.raw_cmd(&create_user);
    }

    let test_user_connection_string = {
        url.set_username("softresetstestuser").unwrap();
        url.set_password(Some("1234batman")).unwrap();
        url.to_string()
    };

    // Check that the test user can't drop databases.
    {
        let test_user_connection = api.block_on(Quaint::new(&test_user_connection_string)).unwrap();

        let err = api
            .block_on(test_user_connection.raw_cmd(&format!(r#"DROP DATABASE `{}`"#, api.test_fn_name())))
            .unwrap_err();

        // insufficient_privilege
        // https://docs.oracle.com/cd/E19078-01/mysql/mysql-refman-5.1/error-handling.html
        assert_eq!(err.original_code().unwrap(), "1044");
    }

    // Check that the soft reset works with migrations, then with schema push.
    {
        let engine = api.new_engine_with_connection_strings(&test_user_connection_string, None);

        engine.reset().send_sync();
        engine.assert_schema().assert_tables_count(0);

        engine
            .schema_push(dm)
            .send()
            .assert_has_executed_steps()
            .assert_green_bang();

        engine.assert_schema().assert_tables_count(1).assert_has_table("Cat");

        engine.reset().send_sync();
        engine.assert_schema().assert_tables_count(0);
    }
}
