use std::{env, fs, io::Write as _, path};

const ROOT_DIR: &str = "tests/simple";

fn main() {
    println!("cargo:rerun-if-changed={ROOT_DIR}");

    let mut all_sql_files = Vec::new();
    find_all_sql_files("", &mut all_sql_files);

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file_path = path::Path::new(&out_dir).join("simple_tests.rs");
    let mut out_file = fs::File::create(out_file_path).unwrap();

    for sql_file in &all_sql_files {
        let test_name = sql_file.trim_start_matches('/').trim_end_matches(".sql");
        let test_name = test_name.replace(['/', '\\'], "_");
        let file_path = sql_file.trim_start_matches('/');
        writeln!(
            out_file,
            "#[test] fn {test_name}() {{ run_simple_test(\"{file_path}\", \"{test_name}\"); }}"
        )
        .unwrap();
    }
}

fn find_all_sql_files(prefix: &str, all_sql_files: &mut Vec<String>) {
    let cargo_manifest_dir = env!("CARGO_MANIFEST_DIR");
    for entry in fs::read_dir(format!("{cargo_manifest_dir}/{ROOT_DIR}/{prefix}")).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let file_name = file_name.to_str().unwrap();
        let entry_path = format!("{prefix}/{file_name}");
        let file_type = entry.file_type().unwrap();

        if file_name == "." || file_name == ".." {
            continue;
        }

        if file_type.is_file() {
            all_sql_files.push(entry_path);
        } else if file_type.is_dir() {
            find_all_sql_files(&entry_path, all_sql_files);
        }
    }
}
