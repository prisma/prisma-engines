use expect_test::expect;
use quaint::{prelude::Queryable, single::Quaint};
use sql_migration_tests::multi_engine_test_api::*;
use test_macros::test_connector;
use user_facing_errors::{UserFacingError, schema_engine::ShadowDbSameAsMainDb};

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
            GRANT USAGE, CREATE ON SCHEMA "public" TO shadowdbconfigtestuser;
            GRANT ALL PRIVILEGES ON DATABASE "testshadowdb0001" TO shadowdbconfigtestuser;
        "#;

        api.raw_cmd(create_user);

        let mut shadow_db_url = url.clone();
        shadow_db_url.set_path("testshadowdb0001");

        let shadow_db_connection = tok(Quaint::new(shadow_db_url.as_ref())).unwrap();

        tok(shadow_db_connection
            .raw_cmd("CREATE SCHEMA IF NOT EXISTS \"public\"; GRANT USAGE, CREATE ON SCHEMA \"public\" TO shadowdbconfigtestuser"))
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
        let err = api
            .new_engine_with_connection_strings_or_err(
                api.connection_string().to_owned(),
                Some(api.connection_string().to_owned()),
            )
            .err()
            .unwrap();

        assert!(err.is_user_facing_error::<ShadowDbSameAsMainDb>(), "{err:?}");
        assert!(err.to_string().contains("The shadow database you configured appears to be the same as the main database. Please specify another shadow database."));
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

/// A shadow database URL that reaches the main database is refused however it is spelled: what
/// counts is the database it denotes, not the string it is written as.
#[track_caller]
fn assert_shadow_db_url_is_refused(api: &TestApi, shadow_db_url: &str) {
    let err = api
        .new_engine_with_connection_strings_or_err(api.connection_string().to_owned(), Some(shadow_db_url.to_owned()))
        .err()
        .unwrap();

    assert!(
        err.is_user_facing_error::<ShadowDbSameAsMainDb>(),
        "shadow database url {shadow_db_url}: {err:?}"
    );
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn differently_spelled_shadow_db_url_must_not_match_main_url_on_postgres(api: TestApi) {
    let main_url = api.connection_string();

    let scheme_alias = main_url.replacen("postgresql://", "postgres://", 1);
    let upper_case_host = main_url.replace("localhost", "LOCALHOST");
    let other_schema = main_url.replace("schema=public", "schema=shadow");
    let extra_parameter = format!("{main_url}&connection_limit=1");

    for spelling in [scheme_alias, upper_case_host, other_schema, extra_parameter] {
        assert_ne!(spelling, main_url);
        assert_shadow_db_url_is_refused(&api, &spelling);
    }
}

#[test_connector(tags(Mysql), exclude(Vitess))]
fn differently_spelled_shadow_db_url_must_not_match_main_url_on_mysql(api: TestApi) {
    let main_url = api.connection_string();

    let upper_case_host = main_url.replace("localhost", "LOCALHOST");
    let extra_parameter = format!("{main_url}?connection_limit=1");

    for spelling in [upper_case_host, extra_parameter] {
        assert_ne!(spelling, main_url);
        assert_shadow_db_url_is_refused(&api, &spelling);
    }
}

#[test_connector(tags(Mssql))]
fn differently_spelled_shadow_db_url_must_not_match_main_url_on_mssql(api: TestApi) {
    let main_url = api.connection_string();

    let upper_case_host = main_url.replace("localhost", "LOCALHOST");
    let other_schema = format!("{main_url};schema=shadow");

    for spelling in [upper_case_host, other_schema] {
        assert_ne!(spelling, main_url);
        assert_shadow_db_url_is_refused(&api, &spelling);
    }
}

#[test_connector(tags(Sqlite))]
fn differently_spelled_shadow_db_path_must_not_match_main_db_path_on_sqlite(api: TestApi) {
    let main_url = api.connection_string();
    let file_path = main_url.trim_start_matches("file:");
    let (directory, file_name) = file_path.rsplit_once('/').unwrap();

    let without_scheme = file_path.to_owned();
    let redundant_current_directory = format!("file:{directory}/./{file_name}");

    for spelling in [without_scheme, redundant_current_directory] {
        assert_ne!(spelling, main_url);
        assert_shadow_db_url_is_refused(&api, &spelling);
    }
}

#[test_connector(tags(Sqlite))]
fn a_relative_shadow_db_path_is_resolved_against_the_working_directory_on_sqlite(api: TestApi) {
    // Connection strings are compared as they are given, and a relative SQLite path in them is
    // resolved against the process's working directory. The bare file name of the main database
    // therefore denotes a different database, unless the main database happens to live in the
    // working directory.
    let file_name = api.connection_string().rsplit_once('/').unwrap().1.to_owned();
    assert!(
        !std::path::Path::new(&file_name).exists(),
        "the test database must not live in the working directory for this test to mean anything"
    );

    api.new_engine_with_connection_strings_or_err(api.connection_string().to_owned(), Some(file_name))
        .map(drop)
        .unwrap();
}

#[test_connector(tags(Postgres, Mysql, Mssql, Sqlite), exclude(CockroachDb, Vitess))]
fn a_separate_shadow_db_on_the_same_server_is_accepted(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let schema = r#"
        model Cat {
            id Int @id
            litterConsumption Int
        }
    "#;

    let mut engine = api.new_engine_with_connection_strings(
        api.connection_string().to_owned(),
        Some(api.create_external_shadow_database()),
    );

    engine
        .create_migration("01init", schema, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(1);
}
