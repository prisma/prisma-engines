use std::fs;

const HELP_TEXT: &str = r#"
Prisma Datamodel v2 formatter

Formats a datamodel v2 file and prints the result to standard output.

USAGE:

    prisma-fmt <INPUT>

<INPUT>: Sets the input datamodel file to use
"#;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() != 1 {
        eprintln!("{}", HELP_TEXT);
    }

    let file_name = &args[0];
    let file_content = fs::read_to_string(&file_name).unwrap_or_else(|_| panic!("Unable to open file {}", file_name));

    datamodel::ast::reformat::Reformatter::new(&file_content).reformat_to(&mut std::io::stdout().lock(), 2);
}
