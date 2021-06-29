use std::fs;

const HELP_TEXT: &str = r#"
Prisma Datamodel v2 to DMMF

Converts a datamodel v2 file to the DMMF JSON representation.

Usage:

    dml-to-dmmf <INPUT>

<INPUT>: Sets the input datamodel file to use
"#;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() != 1 {
        eprintln!("{}", HELP_TEXT);
    }

    let file_name = &args[0];
    let file = fs::read_to_string(&file_name).unwrap_or_else(|_| panic!("Unable to open file {}", file_name));

    let validated = datamodel::parse_datamodel_or_pretty_error(&file, file_name);

    match &validated {
        Err(formatted) => {
            println!("{}", formatted);
        }
        Ok(dml) => {
            let json = datamodel::json::dmmf::render_to_dmmf(&dml.subject);
            println!("{}", json);
        }
    }
}
