use datamodel::ast::reformat::Reformatter;
use std::{
    fs::{self, File},
    io::{self, BufWriter, Read},
};

use crate::FormatOpts;

pub fn run(opts: FormatOpts) {
    let datamodel_string = match opts.input {
        Some(file_name) => {
            fs::read_to_string(&file_name).expect(&format!("Unable to open file {}", file_name.display()))
        }
        None => {
            let mut buf = String::new();

            io::stdin()
                .read_to_string(&mut buf)
                .expect("Unable to read from stdin.");

            buf
        }
    };

    match opts.output {
        Some(file_name) => {
            let file = File::open(&file_name).expect(&format!("Unable to open file {}", file_name.display()));
            let mut stream = BufWriter::new(file);

            Reformatter::new(&datamodel_string).reformat_to(&mut stream, opts.tabwidth);
        }
        None => {
            Reformatter::new(&datamodel_string).reformat_to(&mut io::stdout().lock(), opts.tabwidth);
        }
    }
}
