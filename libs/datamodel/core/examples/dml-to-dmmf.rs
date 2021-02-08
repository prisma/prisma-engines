use clap::{App, Arg};
use datamodel::{diagnostics::Validator, Datamodel};
use std::fs;

fn main() {
    let matches = App::new("Prisma Datamodel v2 to DMMF")
        .version("0.1")
        .author("Emanuel Jöbstl <emanuel.joebstl@gmail.com>")
        .about("Converts a datamodel v2 file to the DMMF JSON representation.")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input datamodel file to use")
                .required(true)
                .index(1),
        )
        .get_matches();

    let file_name = matches.value_of("INPUT").unwrap();
    let file = fs::read_to_string(&file_name).unwrap_or_else(|_| panic!("Unable to open file {}", file_name));

    let validator = Validator::<Datamodel>::new();

    match validator.parse_str(&file) {
        Err(formatted) => {
            println!("{}", formatted.to_pretty_string(file_name, &file));
        }
        Ok(dml) => {
            let json = datamodel::json::dmmf::render_to_dmmf(&dml.subject);
            println!("{}", json);
        }
    }
}
