use crate::{LintOpts, MiniError};
use datamodel::{
    diagnostics::{DatamodelError, DatamodelWarning, Validator},
    Datamodel,
};
use std::io::{self, Read};

pub fn run(opts: LintOpts) {
    let mut datamodel_string = String::new();

    io::stdin()
        .read_to_string(&mut datamodel_string)
        .expect("Unable to read from stdin.");

    let mut validator = Validator::<Datamodel>::new();

    // Let's add all the magic we can in here
    validator.standardize_models();

    if opts.no_env_errors {
        validator.ignore_datasource_urls();
    }

    match validator.parse_str(&datamodel_string) {
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

            print_diagnostics(mini_errors);
        }
        Ok(validated_datamodel) => {
            let mini_warnings: Vec<MiniError> = validated_datamodel
                .warnings
                .into_iter()
                .map(|warn: DatamodelWarning| MiniError {
                    start: warn.span().start,
                    end: warn.span().end,
                    text: format!("{}", warn),
                    is_warning: true,
                })
                .collect();

            print_diagnostics(mini_warnings);
        }
    }
}

fn print_diagnostics(diagnostics: Vec<MiniError>) {
    let json = serde_json::to_string(&diagnostics).expect("Failed to render JSON");

    print!("{}", json)
}
