use std::env;
use std::fs;

use psl::Diagnostics;

fn main() {
    let args: Vec<String> = env::args().collect();
    let in_file = &args[1];
    let out_file = &args[2];

    let in_contents = fs::read_to_string(in_file).expect("Can not read in file");
    let mut diagnostics = Diagnostics::new();
    let ast = psl::schema_ast::parse_schema(&in_contents, &mut diagnostics);
    let encoded = bincode::serialize(&ast).unwrap();

    fs::write(out_file, encoded).unwrap();
}
