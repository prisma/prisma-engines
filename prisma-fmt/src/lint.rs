use crate::{LintOpts, MiniError};
use datamodel::diagnostics::{DatamodelError, DatamodelWarning};

use std::io::{self, Read};

pub fn run(opts: LintOpts) {
    let mut datamodel_string = String::new();

    io::stdin()
        .read_to_string(&mut datamodel_string)
        .expect("Unable to read from stdin.");

    let datamodel_result = if opts.no_env_errors {
        datamodel::parse_datamodel_and_ignore_datasource_urls(&datamodel_string)
    } else {
        datamodel::parse_datamodel(&datamodel_string)
    };

    match datamodel_result {
        Err(err) => {
            let mut mini_errors: Vec<MiniError> = err
                .to_error_iter()
                .map(|err: &DatamodelError| MiniError {
                    start: err.span().start,
                    end: err.span().end,
                    text: format!("{}", err),
                    is_warning: false,
                })
                .collect();

            let mut mini_warnings: Vec<MiniError> = err
                .to_warning_iter()
                .map(|warn: &DatamodelWarning| MiniError {
                    start: warn.span().start,
                    end: warn.span().end,
                    text: format!("{}", warn),
                    is_warning: true,
                })
                .collect();

            mini_errors.append(&mut mini_warnings);

            let json = serde_json::to_string(&mini_errors).expect("Failed to render JSON");

            print!("{}", json)
        }
        _ => print!("[]"),
    }
}
