use std::{env, fs, io::Write as _, path};

const VALIDATIONS_ROOT_DIR: &str = "tests/validation";
const REFORMAT_ROOT_DIR: &str = "tests/reformatter";
const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn main() {
    build_validation_tests();
    build_reformat_tests();
}

fn build_reformat_tests() {
    println!("cargo:rerun-if-changed={REFORMAT_ROOT_DIR}");

    let mut all_schemas = Vec::new();
    find_all_schemas("", &mut all_schemas, REFORMAT_ROOT_DIR);

    let mut out_file = out_file("reformat_tests.rs");
    let schemas_to_reformat = all_schemas.iter().filter(|name| !name.ends_with(".reformatted.prisma"));

    for schema_path in schemas_to_reformat {
        let test_name = test_name(schema_path);
        let file_path = schema_path.trim_start_matches('/');
        writeln!(
            out_file,
            "#[test] fn {test_name}() {{ run_reformat_test(\"{file_path}\"); }}"
        )
        .unwrap();
    }
}

fn build_validation_tests() {
    println!("cargo:rerun-if-changed={VALIDATIONS_ROOT_DIR}");

    let mut all_schemas = Vec::new();
    find_all_schemas("", &mut all_schemas, VALIDATIONS_ROOT_DIR);

    let mut out_file = out_file("validation_tests.rs");

    for schema_path in &all_schemas {
        let test_name = test_name(schema_path);
        let file_path = schema_path.trim_start_matches('/');
        writeln!(
            out_file,
            "#[test] fn {test_name}() {{ run_validation_test(\"{file_path}\"); }}"
        )
        .unwrap();
    }
}

fn find_all_schemas(prefix: &str, all_schemas: &mut Vec<String>, root_dir: &'static str) {
    for entry in fs::read_dir(format!("{CARGO_MANIFEST_DIR}/{root_dir}/{prefix}")).unwrap() {
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
            find_all_schemas(&entry_path, all_schemas, root_dir);
        }
    }
}

fn out_file(name: &str) -> std::fs::File {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file_path = path::Path::new(&out_dir).join(name);
    fs::File::create(out_file_path).unwrap()
}

fn test_name(schema_file_path: &str) -> String {
    schema_file_path
        .trim_start_matches('/')
        .trim_end_matches(".prisma")
        .replace(['/', '\\'], "_")
}
