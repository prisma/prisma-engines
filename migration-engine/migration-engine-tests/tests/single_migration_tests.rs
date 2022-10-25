use migration_engine_tests::test_api::*;
use std::{fs, io::Write as _, path, sync::Arc};
use test_setup::TestApiArgs;

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
    let mut test_api = TestApi::new(test_api_args);
    let source_file = psl::SourceFile::new_allocated(text.clone());

    let migration: String = test_api.connector_diff(
        migration_core::migration_connector::DiffTarget::Empty,
        migration_core::migration_connector::DiffTarget::Datamodel(source_file.clone()),
    );

    test_api.raw_cmd(&migration); // check that it runs

    let second_migration = test_api.connector_diff(
        migration_core::migration_connector::DiffTarget::Database,
        migration_core::migration_connector::DiffTarget::Datamodel(source_file),
    );

    if second_migration != "-- This is an empty migration." {
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
