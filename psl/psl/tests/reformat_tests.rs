mod panic_with_diff;

use std::{fs, io::Write as _, path};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reformatter");

#[inline(never)] // we want to compile fast
fn run_reformat_test(test_file_path: &str) {
    let file_path = path::Path::new(TESTS_ROOT).join(test_file_path);
    let text = fs::read_to_string(&file_path).unwrap();
    let reformatted_text: String = psl::reformat(&text, 2).unwrap();
    let snapshot_file_name = path::Path::new(TESTS_ROOT).join(format!(
        "{}.reformatted.prisma",
        test_file_path.trim_end_matches(".prisma")
    ));
    let expected_text: String = fs::read_to_string(&snapshot_file_name).unwrap_or_default();

    if reformatted_text == expected_text {
        return; // test passed
    }

    if std::env::var("UPDATE_EXPECT").is_ok() {
        let mut file = fs::File::create(&snapshot_file_name).unwrap(); // truncate
        file.write_all(reformatted_text.as_bytes()).unwrap();
    } else {
        panic_with_diff::panic_with_diff(&expected_text, &reformatted_text);
    }
}

include!(concat!(env!("OUT_DIR"), "/reformat_tests.rs"));
