use expect_test::expect;
use quaint::{prelude::Queryable, single::Quaint};
use sql_migration_tests::multi_engine_test_api::*;
use test_macros::test_connector;
use user_facing_errors::UserFacingError;

// exclude: auth works differently in single-node insecure cockroach
#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn shadow_db_url_can_be_configured_on_postgres(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let mut url: url::Url = api.connection_string().parse().unwrap();

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
        {
            let mut engine = api.new_engine();

            engine
                .create_migration("01initcats", dm1, &migrations_directory)
                .send_sync();
        }

        api.raw_cmd("DROP DATABASE IF EXISTS testshadowdb0001");
        api.raw_cmd("CREATE DATABASE testshadowdb0001");

        let create_user = r#"
            DROP USER IF EXISTS shadowdbconfigtestuser;
            CREATE USER shadowdbconfigtestuser PASSWORD '1234batman' LOGIN;
            GRANT USAGE, CREATE ON SCHEMA "prisma-tests" TO shadowdbconfigtestuser;
            GRANT ALL PRIVILEGES ON DATABASE "testshadowdb0001" TO shadowdbconfigtestuser;
        "#;

        api.raw_cmd(create_user);

        let mut shadow_db_url = url.clone();
        shadow_db_url.set_path("testshadowdb0001");

        let shadow_db_connection = tok(Quaint::new(shadow_db_url.as_ref())).unwrap();

        tok(shadow_db_connection.raw_cmd(
            "CREATE SCHEMA \"prisma-tests\"; GRANT USAGE, CREATE ON SCHEMA \"prisma-tests\" TO shadowdbconfigtestuser",
        ))
        .unwrap();
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
        let test_user_connection = tok(Quaint::new(&test_user_connection_string)).unwrap();

        let err = tok(test_user_connection.raw_cmd("CREATE DATABASE shadowdburltest83429")).unwrap_err();

        assert_eq!(err.original_code().unwrap(), "42501"); // insufficient_privilege (https://www.postgresql.org/docs/current/errcodes-appendix.html)
    }

    // Check that commands using the shadow database work.
    {
        let mut engine =
            api.new_engine_with_connection_strings(test_user_connection_string, Some(custom_shadow_db_url));

        engine
            .apply_migrations(&migrations_directory)
            .send_sync()
            .assert_applied_migrations(&["01initcats"]);

        engine
            .create_migration("02addMeowFrequency", dm2, &migrations_directory)
            .send_sync();

        engine
            .apply_migrations(&migrations_directory)
            .send_sync()
            .assert_applied_migrations(&["02addMeowFrequency"]);

        engine
            .assert_schema()
            .assert_tables_count(2)
            .assert_has_table("_prisma_migrations")
            .assert_table("Cat", |table| table.assert_has_column("meowFrequency"));
    }
}

#[test_connector(tags(Postgres))]
fn shadow_db_url_must_not_match_main_url(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let schema = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    // URLs match -> error
    {
        let mut engine = api.new_engine_with_connection_strings(
            api.connection_string().to_owned(),
            Some(api.connection_string().to_owned()),
        );

        let err = engine
            .create_migration("01init", schema, &migrations_directory)
            .send_unwrap_err()
            .to_string();

        assert!(err.contains("The shadow database you configured appears to be the same as the main database. Please specify another shadow database."));
    }

    // Database name is different -> fine
    {
        api.raw_cmd("DROP DATABASE IF EXISTS testshadowdb0002");
        api.raw_cmd("CREATE DATABASE testshadowdb0002");

        let mut url: url::Url = api.connection_string().parse().unwrap();
        url.set_path("/testshadowdb0002");

        let mut engine =
            api.new_engine_with_connection_strings(api.connection_string().to_owned(), Some(url.to_string()));

        engine
            .create_migration("01init", schema, &migrations_directory)
            .send_sync()
            .assert_migration_directories_count(1);
    }
}

#[test_connector(tags(Postgres, Mysql))]
fn shadow_db_not_reachable_error_must_have_the_right_connection_info(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let schema = r#"
        model Cat {
            id Int @id
            litterConsumption Int
            hungry Boolean @default(true)
        }
    "#;

    let mut url: url::Url = api.connection_string().parse().unwrap();
    url.set_port(Some(39824)).unwrap(); // let's assume no database is running on that port

    let mut engine = api.new_engine_with_connection_strings(api.connection_string().to_owned(), Some(url.to_string()));

    let err = engine
        .create_migration("01init", schema, &migrations_directory)
        .send_unwrap_err()
        .to_user_facing();

    let assertion = expect![[r#"
        Can't reach database server at `localhost:39824`

        Please make sure your database server is running at `localhost:39824`."#]];

    assertion.assert_eq(err.message());

    assert_eq!(
        err.unwrap_known().error_code,
        user_facing_errors::common::DatabaseNotReachable::ERROR_CODE
    );
}
