use crate::FormatOpts;
use psl::reformat;
use std::{
    fs::{self, File},
    io::{self, BufWriter, Read, Write as _},
};

pub fn run(opts: FormatOpts) {
    let datamodel_string = match opts.input {
        Some(file_name) => {
            fs::read_to_string(&file_name).unwrap_or_else(|_| panic!("Unable to open file {}", file_name.display()))
        }
        None => {
            let mut buf = String::new();

            io::stdin()
                .read_to_string(&mut buf)
                .expect("Unable to read from stdin.");

            buf
        }
    };

    let reformatted = reformat(&datamodel_string, opts.tabwidth).unwrap_or(datamodel_string);
    match opts.output {
        Some(file_name) => {
            let file = File::open(&file_name).unwrap_or_else(|_| panic!("Unable to open file {}", file_name.display()));
            let mut file = BufWriter::new(file);
            file.write_all(reformatted.as_bytes()).unwrap();
        }
        None => io::stdout().lock().write_all(reformatted.as_bytes()).unwrap(),
    }
}
