use std::env;
use std::fs;

use bincode::DefaultOptions;
use bincode::Options;
use psl::Diagnostics;
use serde::Serialize;

fn main() {
    let args: Vec<String> = env::args().collect();
    let in_file = &args[1];
    let out_file = &args[2];

    let in_contents = fs::read_to_string(in_file).expect("Can not read in file");
    let mut diagnostics = Diagnostics::new();
    let ast = psl::schema_ast::parse_schema(&in_contents, &mut diagnostics);

    let bincode_options = bincode::options().with_varint_encoding();
    bincode_options
        .serialize_into(fs::File::create(out_file).unwrap(), &ast)
        .unwrap()
}
