use connection_string::JdbcString;
use enumflags2::BitFlags;
use indoc::formatdoc;
use psl::PreviewFeature;
use quaint::single::Quaint;
use schema_connector::{CompositeTypeDepth, ConnectorParams, IntrospectionContext, SchemaConnector};
use sql_introspection_tests::test_api::{Queryable, ToIntrospectionTestResult};
use sql_schema_connector::SqlSchemaConnector;
use std::{fs, io::Write as _, path};
use test_setup::{
    mssql::init_mssql_database, mysql::create_mysql_database, postgres::create_postgres_database,
    runtime::run_with_thread_local_runtime as tok, sqlite_test_url,
};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/simple");

fn run_simple_test(test_file_path: &str, test_function_name: &'static str) {
    let file_path = path::Path::new(TESTS_ROOT).join(test_file_path);
    let text = std::fs::read_to_string(&file_path).unwrap();
    let mut lines = text.lines().peekable();

    let preview_features = match lines.peek() {
        Some(s) if s.starts_with("-- preview_features=") => {
            let line = lines.next().expect("Expected file not to be empty.");
            let line = line.trim_start_matches("-- preview_features=");
            let mut preview_features = BitFlags::empty();

            for s in line.split(',').map(|s| s.trim()) {
                match PreviewFeature::parse_opt(s) {
                    Some(feat) => preview_features |= feat,
                    None => panic!("unknown preview feature: {s}"),
                }
            }

            preview_features
        }
        _ => BitFlags::empty(),
    };
    let namespaces = match lines.peek() {
        Some(s) if s.starts_with("-- schemas=") => {
            let line = lines.next().expect("Expected file not to be empty.");
            let line = line.trim_start_matches("-- schemas=");

            Some(line.split(',').map(|s| s.trim()).map(ToString::to_string).collect())
        }
        _ => None,
    };
    let tags = {
        let first_line = lines.next().expect("Expected file not to be empty.");
        let expected_tags_prefix = "-- tags=";
        assert!(
            first_line.starts_with(expected_tags_prefix),
            "The first line of a simple test must start with \"{expected_tags_prefix}\""
        );
        let tags = first_line.trim_start_matches(expected_tags_prefix);
        test_setup::tags_from_comma_separated_list(tags)
    };
    let excluded = {
        let second_line = lines.next().expect("Expected test file not to be empty.");
        let expected_tags_prefix = "-- exclude=";
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

    let mut database_url = std::env::var("TEST_DATABASE_URL").expect(r#"
Missing TEST_DATABASE_URL from environment.

If you are developing with the docker compose based setup, you can find the environment variables under .test_database_urls at the project root.

Example usage:

source .test_database_urls/mysql_5_6
    "#);

    if database_url == "sqlite" {
        database_url = format!("{}.db", sqlite_test_url(test_function_name));

        let file = database_url.trim_start_matches("file:");
        std::fs::remove_file(file).ok();
    }

    let conn = tok(Quaint::new(&database_url)).unwrap();
    let version = tok(conn.version()).unwrap();

    let provider = if version.map(|v| v.contains("CockroachDB")).unwrap_or(false) {
        "cockroachdb"
    } else {
        let provider = database_url
            .find(':')
            .map(|prefix_end| &database_url[..prefix_end])
            .unwrap_or_else(|| database_url.as_str());

        if provider == "file" {
            "sqlite"
        } else {
            provider
        }
    };

    match provider {
        "cockroachdb" | "postgres" | "postgresql" => {
            tok(create_postgres_database(&database_url, test_function_name)).unwrap();
        }
        "mysql" => {
            tok(create_mysql_database(&database_url, test_function_name)).unwrap();
        }
        "sqlserver" => {
            tok(init_mssql_database(&database_url, test_function_name)).unwrap();
        }
        _ => (),
    }

    let database_url = if provider == "sqlserver" {
        let mut jdbc: JdbcString = format!("jdbc:{database_url}").parse().unwrap();

        jdbc.properties_mut()
            .insert("database".to_string(), test_function_name.to_string());

        jdbc.to_string().trim_start_matches("jdbc:").to_string()
    } else if provider == "sqlite" {
        database_url.clone()
    } else {
        format!("{database_url}/{test_function_name}")
    };

    let conn = tok(Quaint::new(&database_url)).unwrap();

    tok(conn.raw_cmd(&text)).unwrap();

    let params = ConnectorParams {
        connection_string: database_url,
        preview_features,
        shadow_database_connection_string: None,
    };

    let mut api = match provider {
        "cockroachdb" => {
            let mut api = SqlSchemaConnector::new_cockroach();
            api.set_params(params).unwrap();
            api
        }
        "postgres" | "postgresql" => {
            let mut api = SqlSchemaConnector::new_postgres();
            api.set_params(params).unwrap();
            api
        }
        "mysql" => {
            let mut api = SqlSchemaConnector::new_mysql();
            api.set_params(params).unwrap();
            api
        }
        "sqlserver" => {
            let mut api = SqlSchemaConnector::new_mssql();
            api.set_params(params).unwrap();
            api
        }
        "sqlite" => {
            let mut api = SqlSchemaConnector::new_sqlite();
            api.set_params(params).unwrap();
            api
        }
        _ => unreachable!(),
    };

    let datasource = formatdoc!(
        r#"
        datasource db {{
            provider = "{provider}"
            url = env("DATABASE_URL")
        }}
    "#
    );

    let generator = if preview_features.is_empty() {
        r#"
            generator js {
                provider = "prisma-client-js"
            }
        "#
        .to_string()
    } else {
        let features = preview_features
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(",");

        formatdoc!(
            r#"
            generator js {{
                provider = "prisma-client-js"
                previewFeatures = [{features}]
            }}
        "#
        )
    };

    let config = format!("{datasource}\n\n{generator}");

    let psl = psl::validate(config.into());

    let ctx = IntrospectionContext::new(psl, CompositeTypeDepth::Infinite, namespaces.clone());

    let introspected = tok(api.introspect(&ctx))
        .map(ToIntrospectionTestResult::to_single_test_result)
        .unwrap_or_else(|err| panic!("{}", err))
        .datamodel;

    let last_comment_idx = text
        .match_indices("/*")
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(text.len() - 1);

    let last_comment = text[last_comment_idx..]
        .trim_start_matches("/*")
        .trim_start_matches('\n')
        .trim_end_matches("*/\n");

    if last_comment == introspected {
        let introspected_schema = match psl::parse_schema(&introspected) {
            Ok(s) => s,
            Err(_err) => {
                eprintln!("The introspected schema is invalid.");
                return; // success! (?)
            }
        };

        let re_introspected = {
            let ctx = IntrospectionContext::new(introspected_schema, CompositeTypeDepth::Infinite, namespaces);

            tok(api.introspect(&ctx))
                .map(ToIntrospectionTestResult::to_single_test_result)
                .unwrap_or_else(|err| panic!("{}", err))
                .datamodel
        };

        if introspected == re_introspected {
            return; // success!
        }

        test_setup::panic_with_diff(&introspected, &re_introspected)
    }

    if std::env::var("UPDATE_EXPECT").is_ok() {
        let mut file = fs::File::create(&file_path).unwrap(); // truncate
        let setup_sql = &text[..last_comment_idx];
        writeln!(file, "{setup_sql}\n/*\n{introspected}*/").unwrap();
        return;
    }

    test_setup::panic_with_diff(last_comment, &introspected);
}

include!(concat!(env!("OUT_DIR"), "/simple_tests.rs"));
