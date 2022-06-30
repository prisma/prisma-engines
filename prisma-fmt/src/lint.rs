use std::borrow::Cow;

use datamodel::diagnostics::{DatamodelError, DatamodelWarning};

#[derive(serde::Serialize)]
pub struct MiniError {
    start: usize,
    end: usize,
    text: String,
    is_warning: bool,
}

pub(crate) fn run(schema: impl Into<Cow<'static, str>>) -> String {
    let datamodel_result = datamodel::lint_schema(schema);

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
        Ok(warnings) => {
            let mini_warnings: Vec<MiniError> = warnings
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

#[cfg(test)]
mod tests {
    use expect_test::expect;
    use indoc::indoc;

    fn lint(s: &'static str) -> String {
        let result = super::run(s);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();

        serde_json::to_string_pretty(&value).unwrap()
    }

    #[test]
    fn deprecated_preview_features_should_give_a_warning() {
        let dml = indoc! {r#"
            datasource db {
              provider = "postgresql"
              url      = env("DATABASE_URL")
            }

            generator client {
              provider = "prisma-client-js"
              previewFeatures = ["createMany"]
            }

            model A {
              id  String   @id
            }
        "#};

        let expected = expect![[r#"
            [
              {
                "start": 149,
                "end": 163,
                "text": "Preview feature \"createMany\" is deprecated. The functionality can be used without specifying it as a preview feature.",
                "is_warning": true
              }
            ]"#]];

        expected.assert_eq(&lint(dml));
    }
}
