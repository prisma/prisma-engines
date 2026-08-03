use expect_test::expect;
use quaint::{prelude::Queryable, single::Quaint};
use sql_migration_tests::{
    multi_engine_test_api::*,
    utils::{query_on, raw_cmd_on},
};
use tempfile::TempDir;
use test_macros::test_connector;
use user_facing_errors::{
    UserFacingError,
    schema_engine::{ShadowDbNotEmpty, ShadowDbSameAsMainDb, ShadowDbTooMuchData},
};

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

const DIRTY_MARKER_TABLE: &str = "CREATE TABLE dirty_marker (id INTEGER PRIMARY KEY)";
const MIGRATIONS_TABLE_ONLY: &str = "CREATE TABLE _prisma_migrations (id VARCHAR(36) PRIMARY KEY)";

/// A shadow database holding something of its own, as a database somebody else uses would.
fn dirty_shadow_database(api: &TestApi, setup_sql: &str) -> String {
    let shadow_db_url = api.create_external_shadow_database();
    raw_cmd_on(&shadow_db_url, setup_sql);
    shadow_db_url
}

/// Plans the second migration of a history, which is the point at which the engine replays the
/// first one into the shadow database.
fn migrations_directory_with_one_migration(api: &TestApi) -> TempDir {
    let migrations_directory = api.create_migrations_directory();

    api.new_engine()
        .create_migration("01init", CAT_SCHEMA, &migrations_directory)
        .send_sync();

    migrations_directory
}

const CAT_SCHEMA: &str = r#"
    model Cat {
        id Int @id
        litterConsumption Int
    }
"#;

const CAT_SCHEMA_WITH_ONE_MORE_FIELD: &str = r#"
    model Cat {
        id Int @id
        litterConsumption Int
        hungry Boolean @default(true)
    }
"#;

#[test_connector(tags(Postgres, Mysql, Mssql, Sqlite), exclude(CockroachDb, Vitess))]
fn a_shadow_db_that_is_not_empty_must_not_be_reset_without_consent(api: TestApi) {
    let migrations_directory = migrations_directory_with_one_migration(&api);
    let shadow_db_url = dirty_shadow_database(&api, DIRTY_MARKER_TABLE);

    let err = api
        .new_engine_with_connection_strings(api.connection_string().to_owned(), Some(shadow_db_url.clone()))
        .create_migration("02hungry", CAT_SCHEMA_WITH_ONE_MORE_FIELD, &migrations_directory)
        .send_unwrap_err();

    assert!(err.is_user_facing_error::<ShadowDbNotEmpty>(), "{err:?}");

    // The whole point of refusing: whatever was in there is still in there.
    api.new_engine_with_connection_strings(shadow_db_url, None)
        .assert_schema()
        .assert_has_table("dirty_marker");
}

#[test_connector(tags(Postgres, Mysql, Mssql, Sqlite), exclude(CockroachDb, Vitess))]
fn a_shadow_db_holding_only_a_migrations_table_must_not_be_reset_without_consent(api: TestApi) {
    let migrations_directory = migrations_directory_with_one_migration(&api);
    let shadow_db_url = dirty_shadow_database(&api, MIGRATIONS_TABLE_ONLY);

    let err = api
        .new_engine_with_connection_strings(api.connection_string().to_owned(), Some(shadow_db_url))
        .create_migration("02hungry", CAT_SCHEMA_WITH_ONE_MORE_FIELD, &migrations_directory)
        .send_unwrap_err();

    assert!(err.is_user_facing_error::<ShadowDbNotEmpty>(), "{err:?}");
}

#[test_connector(tags(Postgres, Mysql, Mssql, Sqlite), exclude(CockroachDb, Vitess))]
fn a_shadow_db_that_is_not_empty_is_reset_with_consent_and_left_empty(api: TestApi) {
    let migrations_directory = migrations_directory_with_one_migration(&api);
    let shadow_db_url = dirty_shadow_database(&api, DIRTY_MARKER_TABLE);

    api.new_engine_with_shadow_db_consent(api.connection_string().to_owned(), Some(shadow_db_url.clone()))
        .create_migration("02hungry", CAT_SCHEMA_WITH_ONE_MORE_FIELD, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(2);

    // The shadow database is left as an empty database, so the next command finds nothing to ask
    // about.
    api.new_engine_with_connection_strings(shadow_db_url, None)
        .assert_schema()
        .assert_tables_count(0);
}

#[test_connector(tags(Sqlite))]
fn a_dirty_sqlite_shadow_db_file_is_refused_and_a_consented_one_is_reset(api: TestApi) {
    let migrations_directory = migrations_directory_with_one_migration(&api);

    // A table the migration history creates too: replaying the history into a file that already
    // holds it fails, because nothing resets an external SQLite shadow database on the way in.
    let shadow_db_url = dirty_shadow_database(&api, "CREATE TABLE Cat (id INTEGER PRIMARY KEY)");

    let err = api
        .new_engine_with_connection_strings(api.connection_string().to_owned(), Some(shadow_db_url.clone()))
        .create_migration("02hungry", CAT_SCHEMA_WITH_ONE_MORE_FIELD, &migrations_directory)
        .send_unwrap_err();

    assert!(err.is_user_facing_error::<ShadowDbNotEmpty>(), "{err:?}");
    api.new_engine_with_connection_strings(shadow_db_url.clone(), None)
        .assert_schema()
        .assert_has_table("Cat");

    api.new_engine_with_shadow_db_consent(api.connection_string().to_owned(), Some(shadow_db_url.clone()))
        .create_migration("02hungry", CAT_SCHEMA_WITH_ONE_MORE_FIELD, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(2);

    api.new_engine_with_connection_strings(shadow_db_url, None)
        .assert_schema()
        .assert_tables_count(0);
}

/// The number of rows a shadow database may hold and still be reset, which the engine refuses to
/// exceed however much consent it is given.
const ROW_COUNT_LIMIT: usize = 1000;

/// A shadow database holding `rows` rows of somebody's data.
///
/// The rows are inserted a few hundred at a time rather than generated by the database, because
/// every flavour spells generating them differently and one of them caps a recursive CTE at a
/// thousand — the number this test is about.
fn shadow_database_with_rows(api: &TestApi, rows: usize) -> String {
    let shadow_db_url = api.create_external_shadow_database();
    raw_cmd_on(&shadow_db_url, "CREATE TABLE big_table (id INTEGER PRIMARY KEY)");

    for chunk in (1..=rows).collect::<Vec<_>>().chunks(500) {
        let values = chunk.iter().map(|id| format!("({id})")).collect::<Vec<_>>().join(",");
        raw_cmd_on(&shadow_db_url, &format!("INSERT INTO big_table (id) VALUES {values}"));
    }

    shadow_db_url
}

fn rows_in_shadow_database(shadow_db_url: &str) -> i64 {
    query_on(shadow_db_url, "SELECT COUNT(*) FROM big_table")
        .into_iter()
        .next()
        .and_then(|row| row.into_iter().next())
        .and_then(|value| value.as_integer())
        .unwrap()
}

#[test_connector(tags(Postgres, Mysql, Mssql, Sqlite), exclude(CockroachDb, Vitess))]
fn a_shadow_db_holding_more_than_the_row_limit_is_refused_even_with_consent(api: TestApi) {
    let migrations_directory = migrations_directory_with_one_migration(&api);
    let shadow_db_url = shadow_database_with_rows(&api, ROW_COUNT_LIMIT + 1);

    let err = api
        .new_engine_with_shadow_db_consent(api.connection_string().to_owned(), Some(shadow_db_url.clone()))
        .create_migration("02hungry", CAT_SCHEMA_WITH_ONE_MORE_FIELD, &migrations_directory)
        .send_unwrap_err();

    assert!(err.is_user_facing_error::<ShadowDbTooMuchData>(), "{err:?}");

    let message = err.to_string();
    assert!(message.contains(&ROW_COUNT_LIMIT.to_string()), "{message}");
    // The location is named the way the engine sanitizes it, without the credentials that reach it.
    assert!(!message.contains(":prisma@"), "{message}");

    // Consent buys a reset, not a deletion: the data is still there.
    assert_eq!(rows_in_shadow_database(&shadow_db_url), (ROW_COUNT_LIMIT + 1) as i64);
}

#[test_connector(tags(Postgres, Mysql, Mssql, Sqlite), exclude(CockroachDb, Vitess))]
fn a_shadow_db_holding_exactly_the_row_limit_is_reset_with_consent(api: TestApi) {
    let migrations_directory = migrations_directory_with_one_migration(&api);
    let shadow_db_url = shadow_database_with_rows(&api, ROW_COUNT_LIMIT);

    api.new_engine_with_shadow_db_consent(api.connection_string().to_owned(), Some(shadow_db_url.clone()))
        .create_migration("02hungry", CAT_SCHEMA_WITH_ONE_MORE_FIELD, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(2);

    // The limit itself is not more than the limit: the shadow database was reset, replayed into,
    // and left empty.
    api.new_engine_with_connection_strings(shadow_db_url, None)
        .assert_schema()
        .assert_tables_count(0);
}
