use crate::offsets::offset_to_lsp_offset;
use psl::diagnostics::{DatamodelError, DatamodelWarning};

#[derive(serde::Serialize)]
pub struct MiniError {
    start: usize,
    end: usize,
    text: String,
    is_warning: bool,
}

pub fn run(schema: &str) -> String {
    let validated_schema = psl::validate(schema.into());
    let diagnostics = &validated_schema.diagnostics;

    let mut mini_errors: Vec<MiniError> = diagnostics
        .errors()
        .iter()
        .map(|err: &DatamodelError| MiniError {
            start: offset_to_lsp_offset(err.span().start, schema),
            end: offset_to_lsp_offset(err.span().end, schema),
            text: err.message().to_string(),
            is_warning: false,
        })
        .collect();

    let mut mini_warnings: Vec<MiniError> = diagnostics
        .warnings()
        .iter()
        .map(|warn: &DatamodelWarning| MiniError {
            start: offset_to_lsp_offset(warn.span().start, schema),
            end: offset_to_lsp_offset(warn.span().end, schema),
            text: warn.message().to_owned(),
            is_warning: true,
        })
        .collect();

    mini_errors.append(&mut mini_warnings);

    print_diagnostics(mini_errors)
}

fn print_diagnostics(diagnostics: Vec<MiniError>) -> String {
    serde_json::to_string(&diagnostics).expect("Failed to render JSON")
}

#[cfg(test)]
mod tests {
    use expect_test::expect;
    use indoc::indoc;

    fn lint(s: &str) -> String {
        let result = super::run(s);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();

        serde_json::to_string_pretty(&value).unwrap()
    }

    #[test]
    fn should_return_utf16_offset() {
        let dml = indoc! {r#"
            // 🌐 ｍｕｌｔｉｂｙｔｅ
            😀
        "#};

        let expected = expect![[r#"
            [
              {
                "start": 16,
                "end": 19,
                "text": "Error validating: This line is invalid. It does not start with any known Prisma schema keyword.",
                "is_warning": false
              }
            ]"#]];

        expected.assert_eq(&lint(dml));
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
