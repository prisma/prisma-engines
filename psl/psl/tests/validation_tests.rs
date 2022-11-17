mod panic_with_diff;

use std::{fs, io::Write as _, path, sync::Arc};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/validation");

#[inline(never)] // we want to compile fast
fn run_validation_test(test_file_path: &str) {
    let file_path = path::Path::new(TESTS_ROOT).join(test_file_path);
    let text = fs::read_to_string(&file_path).unwrap();
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

    let source_file = psl::parser_database::SourceFile::new_allocated(Arc::from(text.into_boxed_str()));
    let validation_result = psl::parse_schema(source_file.clone());

    let errors = match (last_comment_contents.is_empty(), validation_result) {
        (true, Ok(_)) => return, // expected and got a valid schema
        (false, Err(errors)) if last_comment_contents == errors => return, // we expected the errors we got
        (_, Err(errors)) => errors,
        (false, Ok(_)) => String::new(), // expected errors, got none
    };

    if std::env::var("UPDATE_EXPECT").is_ok() {
        let mut file = fs::File::create(&file_path).unwrap(); // truncate

        let schema = last_comment_idx
            .map(|idx| &source_file.as_str()[..idx])
            .unwrap_or(source_file.as_str());
        file.write_all(schema.as_bytes()).unwrap();

        for line in errors.lines() {
            writeln!(file, "// {line}").unwrap();
        }
        return;
    }

    panic_with_diff::panic_with_diff(&last_comment_contents, &errors)
}

include!(concat!(env!("OUT_DIR"), "/validation_tests.rs"));
