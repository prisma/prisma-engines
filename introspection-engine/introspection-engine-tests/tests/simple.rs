use introspection_engine_tests::test_api::TestApi;
use std::{fs, io::Write as _, path};
use test_setup::{runtime::run_with_thread_local_runtime as tok, TestApiArgs};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/simple");

fn run_simple_test(test_file_path: &str, test_function_name: &'static str) {
    let file_path = path::Path::new(TESTS_ROOT).join(test_file_path);
    let text = std::fs::read_to_string(&file_path).unwrap();
    let mut lines = text.lines();

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

    let test_api_args = TestApiArgs::new(test_function_name, &[], &[]);

    if test_setup::should_skip_test(tags, excluded, Default::default()) {
        return;
    }

    let api = tok(TestApi::new(test_api_args));
    tok(api.raw_cmd(&text));
    let introspected = tok(api.introspect()).unwrap_or_else(|err| panic!("{}", err));

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
        return; // success!
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
