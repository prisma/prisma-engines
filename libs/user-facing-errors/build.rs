use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn main() {
    let common = read_file("./src/common.rs");
    let query = read_file("./src/query_engine.rs");
    let files = vec![common, query];

    // Read the files and generate a list of errors for our dmmf
    let error_src = user_facing_error_parsing::parse_files(files);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("user_error_list.rs");

    fs::write(&dest_path, error_src).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}

fn read_file(path: &str) -> String {
    let mut file = File::open(path).expect("Unable to open file");
    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");

    src
}
