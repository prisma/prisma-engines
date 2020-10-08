
use std::fs;

extern crate clap;
use clap::{App, Arg};

fn main() {
    let matches = App::new("Prisma Datamodel v2 formatter")
        .version("0.1")
        .author("Emanuel Jöbstl <emanuel.joebstl@gmail.com>")
        .about("Formats a datamodel v2 file and prints the result to standard output.")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input datamodel file to use")
                .required(true)
                .index(1),
        )
        .get_matches();

    let file_name = matches.value_of("INPUT").unwrap();
    let file_content = fs::read_to_string(&file_name).unwrap_or_else(|_| panic!("Unable to open file {}", file_name));

    datamodel::ast::reformat::Reformatter::new(&file_content).reformat_to(&mut std::io::stdout().lock(), 2);
}
