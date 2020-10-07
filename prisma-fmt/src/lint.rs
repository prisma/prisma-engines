use crate::{LintOpts, MiniMessage};
use datamodel::messages::{DatamodelError, DatamodelWarning};
use serde_json;
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
            let mut mini_warnings: Vec<MiniMessage> = err
                .warnings
                .iter()
                .map(|warn: &DatamodelWarning| MiniMessage {
                    start: warn.span().start,
                    end: warn.span().end,
                    text: format!("{}", warn),
                    error: false,
                })
                .collect();
            let mut mini_errors: Vec<MiniMessage> = err
                .errors
                .iter()
                .map(|err: &DatamodelError| MiniMessage {
                    start: err.span().start,
                    end: err.span().end,
                    text: format!("{}", err),
                    error: true,
                })
                .collect();

            let json = serde_json::to_string(&(mini_errors.append(&mut mini_warnings))).expect("Failed to render JSON");

            print!("{}", json)
        }
        _ => print!("[]"),
    }
}
