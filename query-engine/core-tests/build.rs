use std::{env, fs, io::Write as _, path};

const QUERY_VALIDATIONS_ROOT_DIR: &str = "tests/query_validation_tests";
const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
const TEST_NAME_SUFFIX: &str = ".query.json";

fn main() {
    build_validation_tests();
}

fn build_validation_tests() {
    println!("cargo:rerun-if-changed={QUERY_VALIDATIONS_ROOT_DIR}");

    let mut all_queries = Vec::new();
    find_all_queries("", &mut all_queries, QUERY_VALIDATIONS_ROOT_DIR);

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file_path = path::Path::new(&out_dir).join("query_validation_tests.rs");
    let mut out_file = fs::File::create(out_file_path).unwrap();

    for query_file_path in all_queries {
        let test_name = query_file_path
            .trim_start_matches('/')
            .trim_end_matches(TEST_NAME_SUFFIX)
            .replace(['/', '\\'], "_");
        let file_path = query_file_path.trim_start_matches('/');

        writeln!(
            out_file,
            "#[test] fn {test_name}() {{ run_query_validation_test(\"{file_path}\"); }}"
        )
        .unwrap();
    }
}

fn find_all_queries(prefix: &str, all_queries: &mut Vec<String>, root_dir: &'static str) {
    for entry in fs::read_dir(format!("{CARGO_MANIFEST_DIR}/{root_dir}/{prefix}")).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let file_name = file_name.to_str().unwrap();
        let entry_path = format!("{prefix}/{file_name}");
        let file_type = entry.file_type().unwrap();

        if file_name == "." || file_name == ".." {
            continue;
        }

        if file_type.is_file() && file_name.ends_with(TEST_NAME_SUFFIX) {
            all_queries.push(entry_path);
        } else if file_type.is_dir() {
            find_all_queries(&entry_path, all_queries, root_dir);
        }
    }
}
