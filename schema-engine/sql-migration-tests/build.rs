use std::{env, fs, io::Write as _, path};

const ROOT_DIR: &str = "tests/single_migration_tests";

fn main() {
    println!("cargo:rerun-if-changed={ROOT_DIR}");

    let mut all_schemas = Vec::new();
    find_all_schemas("", &mut all_schemas);

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file_path = path::Path::new(&out_dir).join("single_migration_tests.rs");
    let mut out_file = fs::File::create(out_file_path).unwrap();

    for schema_path in &all_schemas {
        let test_name = schema_path.trim_start_matches('/').trim_end_matches(".prisma");
        let test_name = test_name.replace(['/', '\\'], "_");
        let file_path = schema_path.trim_start_matches('/');
        writeln!(
            out_file,
            "#[test] fn {test_name}() {{ run_single_migration_test(\"{file_path}\", \"{test_name}\"); }}"
        )
        .unwrap();
    }
}

fn find_all_schemas(prefix: &str, all_schemas: &mut Vec<String>) {
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
            all_schemas.push(entry_path);
        } else if file_type.is_dir() {
            find_all_schemas(&entry_path, all_schemas);
        }
    }
}
