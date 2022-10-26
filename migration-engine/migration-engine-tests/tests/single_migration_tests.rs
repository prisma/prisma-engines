use migration_core::migration_connector::{ConnectorParams, MigrationConnector};
use migration_engine_tests::test_api::*;
use sql_migration_connector::SqlMigrationConnector;
use std::{fs, io::Write as _, path, sync::Arc};
use test_setup::{runtime::run_with_thread_local_runtime as tok, TestApiArgs};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/single_migration_tests");

#[inline(never)] // we want to compile fast
fn run_single_migration_test(test_file_path: &str, test_function_name: &'static str) {
    let file_path = path::Path::new(TESTS_ROOT).join(test_file_path);
    let text: Arc<str> = Arc::from(std::fs::read_to_string(&file_path).unwrap().into_boxed_str());

    let last_comment_idx = {
        let mut idx = None;
        let newlines = text.char_indices().filter(|(_, c)| *c == '\n');

        for (newline_idx, _) in newlines {
            match (text.get(newline_idx + 1..newline_idx + 3), idx) {
                (Some("//"), None) => {
                    idx = Some(newline_idx + 1); // new comment
                }
                (Some("//"), Some(_)) => (), // comment continues
                (None, _) => (),             // eof
                (Some(_), _) => {
                    idx = None;
                }
            }
        }

        idx
    };
    let last_comment_contents: String = last_comment_idx
        .map(|idx| {
            let mut out = String::with_capacity(text.len() - idx);
            for line in text[idx..].lines() {
                out.push_str(line.trim_start_matches("// "));
                out.push('\n');
            }
            out
        })
        .unwrap_or_default();

    let mut lines = text.lines();
    let tags = {
        let first_line = lines.next().expect("Expected file not to be empty.");
        let expected_tags_prefix = "// tags=";
        assert!(
            first_line.starts_with(expected_tags_prefix),
            "The first line of a single migration test test must start with \"{}\"",
            expected_tags_prefix
        );
        let tags = first_line.trim_start_matches(expected_tags_prefix);
        test_setup::tags_from_comma_separated_list(tags)
    };
    let excluded = {
        let second_line = lines.next().expect("Expected test file not to be empty.");
        let expected_tags_prefix = "// exclude=";
        if second_line.starts_with(expected_tags_prefix) {
            let tags = second_line.trim_start_matches(expected_tags_prefix);
            test_setup::tags_from_comma_separated_list(tags)
        } else {
            Default::default()
        }
    };

    if test_setup::should_skip_test(tags, excluded, Default::default()) {
        return;
    }

    let test_api_args = TestApiArgs::new(test_function_name, &[], &[]);
    let connection_string = if tags.contains(Tags::Postgres) {
        let (_, _, connection_string) = tok(test_api_args.create_postgres_database());
        connection_string
    } else if tags.contains(Tags::Vitess) {
        let params = ConnectorParams {
            connection_string: test_api_args.database_url().to_owned(),
            preview_features: Default::default(),
            shadow_database_connection_string: None,
        };
        let mut conn = SqlMigrationConnector::new_mysql();
        conn.set_params(params).unwrap();
        tok(conn.reset(false)).unwrap();
        test_api_args.database_url().to_owned()
    } else if tags.contains(Tags::Mysql) {
        let (_, connection_string) = tok(test_api_args.create_mysql_database());
        connection_string
    } else if tags.contains(Tags::Mssql) {
        let (_, connection_string) = tok(test_api_args.create_mssql_database());
        connection_string
    } else if tags.contains(Tags::Sqlite) {
        test_setup::sqlite_test_url(test_api_args.test_function_name())
    } else {
        unreachable!()
    };

    let host = Arc::new(migration_engine_tests::test_api::TestConnectorHost::default());
    let migration_engine = migration_core::migration_api(None, Some(host.clone())).unwrap();

    tok(migration_engine.diff(migration_core::json_rpc::types::DiffParams {
        exit_code: None,
        script: true,
        shadow_database_url: None,
        from: migration_core::json_rpc::types::DiffTarget::Empty,
        to: migration_core::json_rpc::types::DiffTarget::SchemaDatamodel(
            migration_core::json_rpc::types::SchemaContainer {
                schema: file_path.to_str().unwrap().to_owned(),
            },
        ),
    }))
    .unwrap();

    let migration: String = host.printed_messages.lock().unwrap()[0].clone();

    tok(
        migration_engine.db_execute(migration_core::json_rpc::types::DbExecuteParams {
            datasource_type: migration_core::json_rpc::types::DbExecuteDatasourceType::Url(
                migration_core::json_rpc::types::UrlContainer {
                    url: connection_string.clone(),
                },
            ),
            script: migration.clone(),
        }),
    )
    .unwrap(); // check that it runs

    let second_migration_result = tok(migration_engine.diff(migration_core::json_rpc::types::DiffParams {
        exit_code: Some(true),
        script: true,
        shadow_database_url: None,
        from: migration_core::json_rpc::types::DiffTarget::Url(migration_core::json_rpc::types::UrlContainer {
            url: connection_string,
        }),
        to: migration_core::json_rpc::types::DiffTarget::SchemaDatamodel(
            migration_core::json_rpc::types::SchemaContainer {
                schema: file_path.to_str().unwrap().to_owned(),
            },
        ),
    }))
    .unwrap();

    if second_migration_result.exit_code != 0 {
        let second_migration: String = host.printed_messages.lock().unwrap()[1].clone();
        panic!("There is drift. Migration:\n\n{second_migration}");
    }

    if migration == last_comment_contents {
        return; // success!
    }

    if std::env::var("UPDATE_EXPECT").is_ok() {
        let mut file = fs::File::create(&file_path).unwrap(); // truncate

        let schema = last_comment_idx.map(|idx| &text[..idx]).unwrap_or(&text);
        file.write_all(schema.as_bytes()).unwrap();

        for line in migration.lines() {
            writeln!(file, "// {line}").unwrap();
        }
        return;
    }

    test_setup::panic_with_diff(&last_comment_contents, &migration);
}

include!(concat!(env!("OUT_DIR"), "/single_migration_tests.rs"));
