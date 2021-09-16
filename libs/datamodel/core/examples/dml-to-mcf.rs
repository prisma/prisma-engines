use std::fs;

const HELP_TEXT: &str = r#"
Prisma Datamodel v2 to DMMF

Converts a datamodel v2 file to the MCF JSON representation.

USAGE:

    dml-to-mcf <INPUT>

<INPUT>: Sets the input datamodel file to use
"#;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() != 1 {
        eprintln!("{}", HELP_TEXT);
    }

    let file_name = &args[0];
    let file = fs::read_to_string(&file_name).unwrap_or_else(|_| panic!("Unable to open file {}", file_name));

    let res = datamodel::parse_configuration(&file);

    match &res {
        Err(errors) => {
            for error in errors.errors() {
                println!();
                error
                    .pretty_print(&mut std::io::stderr().lock(), file_name, &file)
                    .expect("Failed to write errors to stderr");
            }
        }
        Ok(config) => {
            let json = serde_json::to_string_pretty(&datamodel::json::mcf::config_to_mcf_json_value(config)).unwrap();
            println!("{}", json);
        }
    }
}
