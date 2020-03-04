use crate::{LintOpts, MiniError};
use datamodel::error::DatamodelError;
use serde_json;
use std::io::{self, Read};

pub fn run(opts: LintOpts) {
    let mut datamodel_string = String::new();

    io::stdin()
        .read_to_string(&mut datamodel_string)
        .expect("Unable to read from stdin.");

    let datamodel_result = if opts.no_env_errors {
        datamodel::parse_datamodel_and_ignore_env_errors(&datamodel_string)
    } else {
        datamodel::parse_datamodel(&datamodel_string)
    };

    match datamodel_result {
        Err(err) => {
            let mini_errors: Vec<MiniError> = err
                .errors
                .iter()
                .map(|err: &DatamodelError| MiniError {
                    start: err.span().start,
                    end: err.span().end,
                    text: format!("{}", err),
                })
                .collect();

            let json = serde_json::to_string(&mini_errors).expect("Failed to render JSON");

            print!("{}", json)
        }
        _ => print!("[]"),
    }
}
