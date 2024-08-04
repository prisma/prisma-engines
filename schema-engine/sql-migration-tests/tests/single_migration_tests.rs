use schema_core::{
    json_rpc::types::SchemasContainer,
    schema_connector::{ConnectorParams, SchemaConnector},
};
use sql_migration_tests::test_api::*;
use sql_schema_connector::SqlSchemaConnector;
use std::{fs, io::Write as _, path, sync::Arc};
use test_setup::{runtime::run_with_thread_local_runtime as tok, TestApiArgs};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/single_migration_tests");

#[inline(never)] // we want to compile fast
fn run_single_migration_test(test_file_path: &str, test_function_name: &'static str) {
    let file_path = path::Path::new(TESTS_ROOT).join(test_file_path);
    let text: Arc<str> = Arc::from(std::fs::read_to_string(&file_path).unwrap().into_boxed_str());
    const EXPECTATION_TEXT: &str = "// Expected Migration:";

    // Find the beginning of expectation comment.
    let last_comment_idx = {
        text.char_indices()
            // only look at newlines
            .filter(|(_, c)| *c == '\n')
            // look for the first EXPECTATION_TEXT
            .find_map(|(idx, _)| {
                // ... if there's enough left of the file to look ahead
                text.get(idx + 1..idx + EXPECTATION_TEXT.len() + 1).and_then(|t| {
                    // ... and that text matches the delimiter
                    if t == EXPECTATION_TEXT {
                        Some(idx + 1)
                    } else {
                        None
                    }
                })
            })
    };

    // Contents of the expectation, to compare with the atual migration.
    let last_comment_contents: String = last_comment_idx
        .map(|idx| {
            let mut out = String::with_capacity(text.len() - idx);
            // Skipping the EXPECTATION_TEXT line.
            for line in text[idx..].lines().skip(1) {
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
            "The first line of a single migration test test must start with \"{expected_tags_prefix}\""
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
        let mut conn = SqlSchemaConnector::new_mysql();
        conn.set_params(params).unwrap();
        tok(conn.reset(false, None)).unwrap();
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

    let host = Arc::new(sql_migration_tests::test_api::TestConnectorHost::default());
    let schema_engine = schema_core::schema_api(None, Some(host.clone())).unwrap();

    tok(schema_engine.diff(schema_core::json_rpc::types::DiffParams {
        exit_code: None,
        script: true,
        shadow_database_url: None,
        from: schema_core::json_rpc::types::DiffTarget::Empty,
        to: schema_core::json_rpc::types::DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![schema_core::json_rpc::types::SchemaContainer {
                path: file_path.to_str().unwrap().to_owned(),
                content: text.to_string(),
            }],
        }),
    }))
    .unwrap();

    let migration: String = host.printed_messages.lock().unwrap()[0].clone();

    tok(schema_engine.db_execute(schema_core::json_rpc::types::DbExecuteParams {
        datasource_type: schema_core::json_rpc::types::DbExecuteDatasourceType::Url(
            schema_core::json_rpc::types::UrlContainer {
                url: connection_string.clone(),
            },
        ),
        script: migration.clone(),
    }))
    .unwrap(); // check that it runs

    let second_migration_result = tok(schema_engine.diff(schema_core::json_rpc::types::DiffParams {
        exit_code: Some(true),
        script: true,
        shadow_database_url: None,
        from: schema_core::json_rpc::types::DiffTarget::Url(schema_core::json_rpc::types::UrlContainer {
            url: connection_string,
        }),
        to: schema_core::json_rpc::types::DiffTarget::SchemaDatamodel(SchemasContainer {
            files: vec![schema_core::json_rpc::types::SchemaContainer {
                path: file_path.to_str().unwrap().to_owned(),
                content: text.to_string(),
            }],
        }),
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

        writeln!(file, "{EXPECTATION_TEXT}").unwrap();

        for line in migration.lines() {
            writeln!(file, "// {line}").unwrap();
        }
        return;
    }

    test_setup::panic_with_diff(&last_comment_contents, &migration);
}

include!(concat!(env!("OUT_DIR"), "/single_migration_tests.rs"));
