use datamodel::diagnostics::{DatamodelError, DatamodelWarning};

#[derive(serde::Serialize)]
pub struct MiniError {
    start: usize,
    end: usize,
    text: String,
    is_warning: bool,
}

pub(crate) fn run(schema: &str) -> String {
    let datamodel_result = datamodel::parse_datamodel(schema);

    match datamodel_result {
        Err(err) => {
            let mut mini_errors: Vec<MiniError> = err
                .errors()
                .iter()
                .map(|err: &DatamodelError| MiniError {
                    start: err.span().start,
                    end: err.span().end,
                    text: format!("{}", err),
                    is_warning: false,
                })
                .collect();

            let mut mini_warnings: Vec<MiniError> = err
                .warnings()
                .iter()
                .map(|warn: &DatamodelWarning| MiniError {
                    start: warn.span().start,
                    end: warn.span().end,
                    text: format!("{}", warn),
                    is_warning: true,
                })
                .collect();

            mini_errors.append(&mut mini_warnings);

            print_diagnostics(mini_errors)
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

            print_diagnostics(mini_warnings)
        }
    }
}

fn print_diagnostics(diagnostics: Vec<MiniError>) -> String {
    serde_json::to_string(&diagnostics).expect("Failed to render JSON")
}
