//! Removes fixed tests from the list of tests expected to fail.
//!
//! Run it with `cargo run --bin qc-tests-unfail`, paste the list of tests from
//! the `cargo test` output, and press Ctrl+D.

use std::{
    collections::HashSet,
    fs::File,
    io::{self, BufRead, Write},
    path::PathBuf,
};

fn main() {
    let workspace_root = std::env::var("WORKSPACE_ROOT").expect("WORKSPACE_ROOT environment variable not set");
    let tests_list_path = std::env::var("SHOULD_FAIL_TESTS").expect("SHOULD_FAIL_TESTS environment variable not set");
    let tests_list_path = PathBuf::from(workspace_root).join(tests_list_path);

    let tests_to_remove = io::stdin()
        .lock()
        .lines()
        .map(|l| l.unwrap().trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect::<HashSet<_>>();

    let tests_list_content = std::fs::read_to_string(&tests_list_path).unwrap();
    let filtered_tests = tests_list_content
        .lines()
        .filter(|line| !tests_to_remove.contains(line.trim()));

    let out_file = File::options()
        .write(true)
        .truncate(true)
        .open(tests_list_path)
        .unwrap();

    let mut writer = io::BufWriter::new(out_file);

    for test in filtered_tests {
        writeln!(writer, "{test}").unwrap();
    }
}
